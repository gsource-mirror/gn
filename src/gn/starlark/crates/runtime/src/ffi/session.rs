// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::PathBuf;

use crate::ffi;
use crate::StarlarkSession;

#[no_mangle]
pub unsafe extern "C" fn new_starlark_session(
    abs_path: &str,
    rel_path: &str,
) -> *mut StarlarkSession {
    let session = Box::new(StarlarkSession::new(
        PathBuf::from(abs_path),
        rel_path.to_owned(),
    ));
    Box::into_raw(session)
}

#[no_mangle]
pub unsafe extern "C" fn free_starlark_session(ptr: *mut StarlarkSession) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

#[no_mangle]
pub unsafe extern "C" fn starlark_session_load(
    session: &StarlarkSession,
    path: &str,
    source_dir: &ffi::SourceDir,
    scope: *mut ffi::Scope,
    origin: *const ffi::ParseNode,
    err: &mut ffi::Err,
) -> *const starlark::environment::FrozenModule {
    let res = (|| -> Result<*const starlark::environment::FrozenModule, String> {
        let package = source_dir.as_rust().to_owned();
        let module = session
            .load(path, &package, scope, origin)
            .map_err(|e: starlark::Error| e.to_string())?;
        let raw_ptr = &*module as *const starlark::environment::FrozenModule;
        Ok(raw_ptr)
    })();

    ffi::handle_result_with_message(err, origin, "Failed to load Starlark file.", res)
}
