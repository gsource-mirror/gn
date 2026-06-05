// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::sync::atomic::{AtomicUsize, Ordering};

use starlark::environment::Module;

/// Like std::mem::transmute, but can only affect lifetime.
///
/// # Safety
///
/// The caller must ensure that the returned reference is not used after the
/// underlying data is dropped.
pub unsafe fn extend_lifetime<'to, T: ?Sized>(val: &T) -> &'to T {
    // Safety: Transmuting lifetime of reference is unsafe, safety is guaranteed by
    // the caller.
    unsafe { std::mem::transmute(val) }
}

static MODULE_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// Returns a unique module ID for the given module.
pub fn get_module_id<'v>(module: &Module<'v>) -> usize {
    if let Some(val) = module.extra_value() {
        if let Some(id) = val.unpack_i32() {
            return id as usize;
        }
    }
    let id = MODULE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    module.set_extra_value(module.heap().alloc(id as i32));
    id
}
