use std::path::PathBuf;

const LINKER_SCRIPT_NAME: &str = "linker.kernel.ld";

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let linker_script_path = PathBuf::from(manifest_dir).join(LINKER_SCRIPT_NAME);

    println!("cargo:rustc-link-arg=-T{}", linker_script_path.display());
    println!("cargo:rerun-if-changed={}", LINKER_SCRIPT_NAME);
}
