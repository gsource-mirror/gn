// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

declare_opaque_type!(OutputFile);

impl OutputFile {
    pub fn as_rust(&self) -> types::File {
        extern "C" {
            fn GetOutputFilePath(file: &OutputFile) -> &'static str;
        }
        types::File::from_cxx(unsafe { GetOutputFilePath(self) })
    }
}
