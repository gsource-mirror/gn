// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::ffi;
use crate::ffi::value::to_cxx_value;
use crate::session::{EvalContext, EvalKind};

#[no_mangle]
pub unsafe extern "C" fn value_from_module(
    module: &starlark::environment::FrozenModule,
    name: &str,
    out: *mut ffi::GnValue,
    scope: *mut ffi::Scope,
    origin: *const ffi::ParseNode,
    err: &mut ffi::Err,
) {
    let res = (|| -> Result<(), String> {
        let v = module.get(name).map_err(|e| e.to_string())?;
        let extra = EvalContext::new(scope, origin, EvalKind::VariableConversion);
        to_cxx_value(v.value(), &mut *out, v.owner(), &extra).map_err(|e| e.err_msg)?;
        Ok(())
    })();

    ffi::handle_result(err, origin, res);
}
