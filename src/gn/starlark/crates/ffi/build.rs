// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

fn main() {
    println!("cargo:rustc-check-cfg=cfg(string_view_ptr_first)");
    // When running with ninja, it should set NINJA_OUT_DIR.
    // If running directly with cargo, we know nothing about the output directory,
    // so we fall back to assuming the default one.
    let out_dir = if let Ok(out_dir) = std::env::var("NINJA_OUT_DIR") {
        std::path::PathBuf::from(out_dir)
    } else {
        let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        manifest_dir.join("../../../../../out")
    };
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=gn_lib");
    println!("cargo:rustc-link-lib=static=base");

    if detect_string_view_ptr_first(&out_dir) {
        println!("cargo:rustc-cfg=string_view_ptr_first");
    }
}

fn detect_string_view_ptr_first(out_dir: &std::path::Path) -> bool {
    if let Ok(content) = std::fs::read_to_string(out_dir.join("rust_config.json")) {
        content.contains("\"string_view_ptr_first\": true")
    } else {
        false
    }
}
