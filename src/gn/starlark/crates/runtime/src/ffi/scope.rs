// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::ffi::{GnValue, Pair, Settings, SourceDir};
use crate::StarlarkSession;

declare_opaque_type!(Scope);

impl Scope {
    pub fn settings(&self) -> &Settings {
        extern "C" {
            fn GetSettingsFromScope(scope: &Scope) -> &Settings;
        }
        unsafe { GetSettingsFromScope(self) }
    }

    pub fn starlark_session(&self) -> &StarlarkSession {
        extern "C" {
            fn GetStarlarkSessionFromScope(scope: &Scope) -> &StarlarkSession;
        }
        unsafe { GetStarlarkSessionFromScope(self) }
    }

    pub fn source_dir(&self) -> &SourceDir {
        extern "C" {
            fn GetScopeSourceDir(scope: &Scope) -> &SourceDir;
        }
        unsafe { GetScopeSourceDir(self) }
    }

    pub fn set_source_dir(&mut self, dir: &str) {
        extern "C" {
            fn SetSourceDir(scope: &mut Scope, dir: &str);
        }
        unsafe {
            SetSourceDir(self, dir);
        }
    }

    pub fn collect_to_kwargs<'a>(&self) -> Vec<Pair<&'a str, *const GnValue>> {
        extern "C" {
            fn CollectScopeToKwargs<'a>(
                scope: &Scope,
                out: *mut Pair<&'a str, *const GnValue>,
                max_len: usize,
            ) -> usize;
        }
        unsafe {
            let len = CollectScopeToKwargs(self, std::ptr::null_mut(), 0);
            let mut out = Vec::with_capacity(len);
            CollectScopeToKwargs(self, out.as_mut_ptr(), len);
            out.set_len(len);
            out
        }
    }
}
