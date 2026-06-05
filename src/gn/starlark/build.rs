// Clang spends ~60 seconds looking for libclang.so for some reason, so
// providing it reduces compilation time by ~60 seconds.
fn libclang_path() -> Option<String> {
    if std::env::var("LIBCLANG_PATH").is_ok() {
        return None;
    }
    let clang = std::env::var("CLANG_PATH").unwrap_or_else(|_| "clang++".to_string());
    let output = std::process::Command::new(clang)
        .arg("-print-resource-dir")
        .output()
        .ok()?;
    if output.status.success() {
        if let Ok(resource_dir) = String::from_utf8(output.stdout) {
            let resource_path = std::path::Path::new(resource_dir.trim());
            // Go up two levels: e.g. /usr/lib/llvm-19/lib/clang/19 -> /usr/lib/llvm-19/lib
            let lib_path = resource_path.parent()?.parent()?;
            if lib_path.exists() {
                return Some(lib_path.to_str()?.to_string());
            }
        }
    }
    None
}

fn main() {
    if let Some(clang_libdir) = libclang_path() {
        std::env::set_var("LIBCLANG_PATH", clang_libdir);
    }

    let debug = std::env::var("PROFILE").unwrap_or_default() == "debug";
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    let mut clang_args = vec!["-std=c++20", "-DRUST"];
    if debug && target_os != "windows" {
        clang_args.push("-D_GLIBCXX_DEBUG=1");
        clang_args.push("-D_LIBCPP_DEBUG=1");
    }

    let mut builder = autocxx_build::Builder::new("src/ffi/bindings.rs", &["../.."])
        .extra_clang_args(&clang_args)
        .build()
        .unwrap();
    builder
        .file("../ffi/cxx_api.cc")
        .file("src/starlark_test_helper.cc")
        .cpp(true)
        .std("c++20")
        .include("../..")
        .define("RUST", "");

    if debug && target_os != "windows" {
        builder.define("_GLIBCXX_DEBUG", "1");
        builder.define("_LIBCPP_DEBUG", "1");
    }

    builder.compile("gn_starlark_rust_bridge");

    // Format the generated file using rustfmt for readability.
    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        let gen_file = std::path::PathBuf::from(out_dir)
            .join("autocxx-build-dir")
            .join("rs")
            .join("autocxx-ffi-default-gen.rs");
        if gen_file.exists() {
            let _ = std::process::Command::new("rustfmt").arg(gen_file).status();
        }
    }

    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=../ffi/cxx_api.h");
    println!("cargo:rerun-if-changed=../ffi/cxx_api.cc");
    println!("cargo:rerun-if-changed=src/starlark_test_helper.cc");
}
