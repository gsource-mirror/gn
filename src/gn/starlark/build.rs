fn main() {
    cxx_build::bridge("src/lib.rs")
        .flag("-std=c++20")
        .include("../..")
        .compile("gn_starlark_rust_bridge");
}
