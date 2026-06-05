// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::ffi::{AsRust, ToRust};

impl<'a> AsRust for &'a cxx::CxxString {
    type Target = &'a str;
    fn as_rust(self) -> &'a str {
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }
}

impl ToRust<String> for cxx::CxxString {
    fn to_rust(&self) -> String {
        self.as_rust().to_owned()
    }
}
