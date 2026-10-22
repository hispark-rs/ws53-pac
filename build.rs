use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=device.x");
    if env::var_os("CARGO_FEATURE_RT").is_some() {
        let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
        fs::write(out.join("device.x"), include_bytes!("device.x")).unwrap();
        println!("cargo:rustc-link-search={}", out.display());
    }
}
