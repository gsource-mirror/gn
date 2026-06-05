// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use strum::{Display, EnumIter, IntoStaticStr};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Display, IntoStaticStr, Allocative)]
#[strum(serialize_all = "snake_case")]
/// Copied from target.h's OutputType enum.
pub enum OutputType {
    // Unknown = 0
    Group = 1,
    Executable,
    SharedLibrary,
    LoadableModule,
    StaticLibrary,
    SourceSet,
    Copy,
    Action,
    ActionForEach,
    BundleData,
    CreateBundle,
    GeneratedFile,
    RustLibrary,
    RustProcMacro,
}

impl OutputType {
    pub fn fields(&self) -> (&'static [&'static str], usize) {
        match self {
            Self::Executable
            | Self::SharedLibrary
            | Self::LoadableModule
            | Self::StaticLibrary
            | Self::SourceSet => (&["sources", "public", "deps", "public_deps"], 2),
            _ => (&[], 0),
        }
    }
}
