// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub trait CxxStringExt {
    fn as_str(&self) -> &str;
}

impl CxxStringExt for cxx::CxxString {
    fn as_str(&self) -> &str {
        self.to_str().unwrap()
    }
}
