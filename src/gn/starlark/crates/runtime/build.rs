fn main() {
    let out_dir = if let Ok(out_dir) = std::env::var("NINJA_OUT_DIR") {
        std::path::PathBuf::from(out_dir)
    } else {
        let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        manifest_dir.join("../../../../../out")
    };
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=test_support");
    println!("cargo:rustc-link-lib=static=gn_lib");
    println!("cargo:rustc-link-lib=static=base");
    println!("cargo:rustc-link-lib=static=string_atom");
}
