// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::PathBuf;
use std::pin::Pin;

use crate::ffi::bindings as ffi;
use crate::ffi::{AsCxx, AsRust, IntoCxx, IntoRust};
pub use crate::session::StarlarkSession;



#[no_mangle]
// Safe because we're using rust::Str in C++.
#[allow(improper_ctypes_definitions)]
pub unsafe extern "C" fn new_starlark_session(abs_path: &str, rel_path: &str) -> *mut ffi::rust::StarlarkSession {
    let session = Box::new(StarlarkSession::new(PathBuf::from(abs_path), PathBuf::from(rel_path)));
    session.into_cxx()
}

#[no_mangle]
// Safe because we're using rust::Str in C++.
#[allow(improper_ctypes_definitions)]
pub unsafe extern "C" fn free_starlark_session(ptr: *mut ffi::rust::StarlarkSession) {
    let _: Option<Box<StarlarkSession>> = ptr.into_rust();
}



#[no_mangle]
// Safe because we're using rust::Str in C++
#[allow(improper_ctypes_definitions)]
pub unsafe extern "C" fn starlark_session_load(
    session: &ffi::rust::StarlarkSession,
    path: &str,
    source_dir: &ffi::SourceDir,
    scope: *mut ffi::Scope,
    origin: *const ffi::ParseNode,
    err: Pin<&mut ffi::Err>,
) -> *const ffi::rust::StarlarkModule {
    let res = (|| -> Result<*const ffi::rust::StarlarkModule, String> {
        let session = session.as_rust();
        let package = <&crate::PackageRef>::from(source_dir);
        let module = session.load(path, package, scope, origin).map_err(|e| e.to_string())?;

        let raw_ptr = Some(module).as_cxx();
        Ok(raw_ptr)
    })();

    crate::ffi::handle_result_with_message(err, origin, "Failed to load Starlark file.", res)
}

static TEST_TOOLCHAIN: std::sync::atomic::AtomicPtr<std::ffi::c_void> = std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

#[no_mangle]
pub unsafe extern "C" fn starlark_session_get_test_toolchain(
    _loader: &ffi::rust::StarlarkSession,
) -> *const std::ffi::c_void {
    TEST_TOOLCHAIN.load(std::sync::atomic::Ordering::Relaxed)
}
