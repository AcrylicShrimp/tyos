use std::path::PathBuf;

const LINKER_SCRIPT_NAME: &str = "linker.user.ld";

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("none") {
        return;
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let linker_script_path = PathBuf::from(manifest_dir).join(LINKER_SCRIPT_NAME);

    println!("cargo:rustc-link-arg=-T{}", linker_script_path.display());
    println!("cargo:rerun-if-changed={}", LINKER_SCRIPT_NAME);
}
