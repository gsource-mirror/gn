// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::declare_opaque_type;

declare_opaque_type!(pub OutputFile);

impl OutputFile {
    /// Converts a GN OutputFile to a starlark File.
    pub fn to_rust(&self) -> types::File {
        extern "C" {
            // Returned OutputFile objects will be owned by targets,
            // which live forever, so &'static is fine.
            fn GetOutputFilePath(file: &OutputFile) -> &'static str;
        }
        // Safety: Just an FFI function.
        let s = unsafe { GetOutputFilePath(self) };
        types::File::new(s)
    }
}
