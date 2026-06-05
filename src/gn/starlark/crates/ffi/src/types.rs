// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use cxx::CxxString;

pub trait CxxStringExt {
    fn as_str(&self) -> &str;
}

impl CxxStringExt for CxxString {
    fn as_str(&self) -> &str {
        // Safety: was a valid C++ string.
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }
}
