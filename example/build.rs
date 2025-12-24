use std::env;
use std::path::PathBuf;

fn main() {
    let src_dir = PathBuf::from("src");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let final_providers_path = out_dir.join("providers.json");

    if let Err(e) = wire_build::generate(&src_dir, &final_providers_path) {
        panic!("wire-build failed to run: {}", e);
    }

    // Tell cargo to re-run the build script if any file in `src` changes.
    println!("cargo:rerun-if-changed=src");
}
