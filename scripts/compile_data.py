"""
This script compiles the files in the data directory into their
optimized form for the final container build.

It is intended to run in the container's build stage.

Dependencies:
* requests
* pillow
* pillow-avif-plugin
"""
import json
import math
import sys
from io import BytesIO
from pathlib import Path

from PIL import Image
from PIL.Image import Resampling
# Linters may show this as dead code, but the import registers a
# handler in the PIL library that enables avif support.
import pillow_avif
import urllib.parse
import urllib.error
import urllib.request
import subprocess
import requests

# How many entries are encoded in the horizontal direction in a spritesheet.
SPRITESHEET_WIDTH = 64

def make_name_id(name: str) -> str:
    """Convert a name to a path."""
    return name.lower().replace(" ", "-")

def make_cache_name_from_url(url: str) -> str:
    REPLACEMENTS = [
        ("http://", ""),
        ("https://", ""),
        (".", "_"),
        ("/", "_")
    ]
    for r in REPLACEMENTS:
        url = url.replace(*r)
    return url

def load_image_from_path(path: str) -> Image:
    try:
        # If the image locator can be parsed as a URL,
        # download the image at that URL and use it.
        urllib.parse.urlparse(path)
        cache_name = make_cache_name_from_url(path)
        cache_path = Path(f".cache/images/{cache_name}")
        if cache_path.exists():
            return Image.open(f".cache/images/{cache_name}")
        print(f"Fetching {path}")
        response = requests.get(path)
        cache_path.parent.mkdir(parents=True, exist_ok=True)
        cache_path.write_bytes(response.content)
        return Image.open(BytesIO(response.content))
    except urllib.error.URLError:
        # If the image locator is not an URL, use a local file.
        return Image.open(f"images/{path}")

def create_spritesheet(path: Path, resolution: tuple[int, int]) -> dict[str, int]:
    """
    Compile a spritesheet from a list of images.
    Returns a map from the image name to its index in the spritesheet.
    """

    # Parse info about the spritesheet
    pokedex = json.loads(path.read_text())
    # Useful to determine how to resize the images.
    # For pixelart we use nearest interpolation, for non-pixelart
    # we use bicubic interpolation.
    is_pixelart = pokedex.get("uses_pixelart_graphics", False)
    images = []
    for entry in pokedex["entries"]:
        if entry is None:
            continue
        if "sprite" in entry:
            if not entry["sprite"] in images:
                images.append(entry["sprite"])
        else:
            if not "https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/home/0.png" in images:
                images.append("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/home/0.png")

    index = 0
    res = {}
    final_width = resolution[0] * min(len(images), SPRITESHEET_WIDTH)
    final_height = resolution[1] * math.ceil(len(images) / SPRITESHEET_WIDTH)
    out_image = Image.new("RGBA", (final_width, final_height))
    for image_path in images:
        image = load_image_from_path(image_path)
        res[image_path] = index
        image = image.resize(resolution, resample=Resampling.NEAREST if is_pixelart else Resampling.BICUBIC)
        out_image.paste(image, ((index % SPRITESHEET_WIDTH) * resolution[0], int(index / SPRITESHEET_WIDTH) * resolution[1]))
        index += 1

    out_path = Path(f"web/images/{make_name_id(pokedex['name'])}-spritesheet.avif")
    if not out_path.parent.exists():
        out_path.parent.mkdir(parents=True)
    out_image.save(out_path)
    return res

def create_thumbnail(path: Path):
    """
    Compile a thumbnail for a pokedex.
    This is a PNG file that contains the pokedex name,
    the date it was last updated, and an image of the pokedex.
    """
    pokedex = json.loads(path.read_text())
    image = load_image_from_path(pokedex["thumbnail"])
    image = image.resize((128, 128))
    image.save(f"web/images/{make_name_id(pokedex['name'])}-thumbnail.avif")

def create_update_record(path: Path, sprite_indices: dict[str, int]):
    """
    Compile an update record for a pokedex.
    This is a JSON file that contains the pokedex name,
    the date it was last updated, and an SQL string to update
    the database to the latest pokedex version.
    """
    pokedex = json.loads(path.read_text())
    git_result = subprocess.run(
        f"git rev-list -1 HEAD -- {path}",
        shell=True,
        stdout=subprocess.PIPE,
    )
    commit_hash = git_result.stdout.decode().strip()

    entries = []
    for entry in pokedex["entries"]:
        if entry is None:
            entries.append(None)
            continue
        entries.append({
            "id": entry["id"],
            "name": entry["name"],
            "form": entry.get("form"),
            "sprite": sprite_indices[entry["sprite"]],
            "shiny": entry.get("shiny", False),
            "gmax": entry.get("gmax", False),
            "technical": entry.get("technical", False),
        })

    jsn = {
        "id": make_name_id(pokedex["name"]),
        "name": pokedex["name"],
        "description": pokedex["description"],
        # Counts the number of non-null entries.
        # This is used for the progress bar.
        "num_entries": sum(1 for x in pokedex["entries"] if x is not None),
        "thumbnail_url": f"/image/{make_name_id(pokedex['name'])}-thumbnail.avif",
        "spritesheet_url": f"/image/{make_name_id(pokedex['name'])}-spritesheet.avif",
        "commit_hash": commit_hash,
        "entries": entries,
    }
    out_path = Path(f"data/pokedexes/{make_name_id(pokedex['name'])}.json")
    if not out_path.parent.exists():
        out_path.parent.mkdir(parents=True)
    out_path.write_text(json.dumps(jsn))

def sanity_check_pokedex_definitions():
    """Sanity check the pokedex definitions for common mistakes."""

    # Global checks
    names = []
    for path in Path("pokedexes").glob("*.json"):
        pokedex = json.loads(path.read_text())
        names.append(pokedex["name"])
    assert len(set(names)) == len(names), "Non-unique pokedex name"

    # Individual checks
    for path in Path("pokedexes").glob("*.json"):
        pokedex = json.loads(path.read_text())
        assert len(pokedex["entries"]) > 0, "Empty pokedex"
        ids = list(x["id"] for x in pokedex["entries"] if x is not None)
        assert len(set(ids)) == len(ids), "Non-unique IDs in pokedex"


if __name__ == "__main__":
    if len(sys.argv) > 1:
        paths = [Path(p) for p in sys.argv[1:]]
    else:
        paths = Path("pokedexes").glob("*.json")
    Path("data/compiled").mkdir(exist_ok=True)
    sanity_check_pokedex_definitions()
    for pokedex in paths:
        print(f"Compiling spritesheet for {pokedex}")
        sprite_indices = create_spritesheet(pokedex, (64, 64))
        print(f"Creating thumbnail for {pokedex}")
        create_thumbnail(pokedex)
        print(f"Compiling pokedex for {pokedex}")
        create_update_record(pokedex, sprite_indices)
