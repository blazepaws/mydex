"""Simple script to create a basic home dex definition from pokeapi"""
from pathlib import Path

import requests
import json

def create_entries() -> list:
    """Create entries for each species in the home dex"""
    entries = []
    national_dex = requests.get("https://pokeapi.co/api/v2/pokedex/national/").json()
    print(f"Loading {len(national_dex['pokemon_entries'])} species")

    id = 0
    for species in national_dex["pokemon_entries"]:
        species = requests.get(species["pokemon_species"]["url"]).json()
        name = [x for x in species["names"] if x["language"]["name"] == "en"][0]["name"]
        print(f"Loading {name}")
        pokemon = [x for x in species["varieties"] if x["is_default"]][0]["pokemon"]
        pokemon = requests.get(pokemon["url"]).json()
        sprite = pokemon["sprites"]["other"]["home"]["front_default"]

        entries.append({
            "id": id,
            "name": name,
            "sprite": sprite,
        })
        id += 1

    return entries

if __name__ == "__main__":
    home_dex = {
        "name": "Home",
        "description": "Species Pokédex for Pokémon Home. Contains one of each species.",
        "thumbnail": "https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/home/25.png",
        "entries": create_entries(),
    }
    Path("data/pokedex/national.json").write_text(json.dumps(home_dex, indent=4))