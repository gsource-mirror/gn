// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::pin::Pin;

use crate::ffi::bindings as ffi;
use crate::session::EvalContext;
use super::starlark_value::to_cxx_value;

#[no_mangle]
// Safe because we're using rust::Str in C++
#[allow(improper_ctypes_definitions)]
pub unsafe extern "C" fn value_from_module(
    module: &ffi::rust::StarlarkModule,
    name: &cxx::CxxString,
    out: *mut ffi::Value,
    scope: *mut ffi::Scope,
    origin: *const ffi::ParseNode,
    err: std::pin::Pin<&mut ffi::Err>,
) {
    use crate::ffi::AsRust;
    let rust_module = module.as_rust();
    let name_str = name.to_str().unwrap();
    let res = (|| -> Result<(), String> {
        let v = rust_module.get(name_str).map_err(|e| e.to_string())?;
        let out_pin = Pin::new_unchecked(&mut *out);
        let extra = EvalContext::new(
            scope,
            origin,
            crate::session::EvalKind::VariableConversion,
        );
        to_cxx_value(v.value(), out_pin, v.owner(), &extra).map_err(|e| e.err_msg)?;
        Ok(())
    })();

    crate::ffi::handle_result(err, origin, res);
}
