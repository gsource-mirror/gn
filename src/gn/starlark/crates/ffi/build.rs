// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

fn main() {
    // When running with ninja, it should set NINJA_OUT_DIR.
    // If running directly with cargo, we know nothing about the output directory,
    // so we fall back to assuming the default one.
    if std::env::var("NINJA_OUT_DIR").is_err() {
        let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        std::env::set_var("NINJA_OUT_DIR", manifest_dir.join("../../../../../out"));
    }
    let out_dir = std::path::PathBuf::from(std::env::var("NINJA_OUT_DIR").unwrap());

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=gn_lib");
    println!("cargo:rustc-link-lib=static=base");

    println!("cargo:rustc-env=NINJA_OUT_DIR={}", out_dir.display());
}
