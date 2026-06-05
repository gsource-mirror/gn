// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::ffi::err::Err;
use crate::ffi::ParseNode;

/// Helper to handle `Result` inside FFI endpoints, populating a C++ `Err` object and returning a default value on failure.
pub fn handle_result<T: Default, E: std::fmt::Display>(
    err: &mut Err,
    origin: *const ParseNode,
    result: Result<T, E>,
) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            let e_str = e.to_string();
            // origin is *const ParseNode, which we map to Option<&ParseNode>
            let origin_ref = unsafe { origin.as_ref() };
            err.populate_with_location(e_str.as_str(), origin_ref);
            T::default()
        }
    }
}

/// Helper to handle `Result` inside FFI endpoints with a custom high-level help message on failure.
pub fn handle_result_with_message<T: Default, E: std::fmt::Display>(
    err: &mut Err,
    origin: *const ParseNode,
    msg: &str,
    result: Result<T, E>,
) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            let e_str = e.to_string();
            let origin_ref = unsafe { origin.as_ref() };
            err.populate_with_help(msg, e_str.as_str(), origin_ref);
            T::default()
        }
    }
}
