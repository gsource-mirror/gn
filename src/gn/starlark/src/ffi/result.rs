// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::ffi::bindings as ffi;

pub fn handle_result<T: Default, E: std::fmt::Display>(
    err: std::pin::Pin<&mut ffi::Err>,
    origin: *const ffi::ParseNode,
    result: Result<T, E>,
) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            let e_str = e.to_string();
            unsafe {
                crate::ffi::PopulateErrWithLocation(err, e_str.as_str(), origin);
            }
            T::default()
        }
    }
}

pub fn handle_result_with_message<T: Default, E: std::fmt::Display>(
    err: std::pin::Pin<&mut ffi::Err>,
    origin: *const ffi::ParseNode,
    msg: &str,
    result: Result<T, E>,
) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            let e_str = e.to_string();
            unsafe {
                crate::ffi::PopulateErrWithHelp(err, msg, e_str.as_str(), origin);
            }
            T::default()
        }
    }
}
