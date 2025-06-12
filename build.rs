fn main() {
    // Tell cargo that the migrations are considered as sources.
    // This prevents it from using a cached version if a migration
    // changes but no rust code is changed.
    println!("cargo:rerun-if-changed=migrations");
}