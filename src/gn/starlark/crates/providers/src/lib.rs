// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod builtin_providers;
pub mod errors;
pub mod extractor;
pub mod provider;

pub use builtin_providers::{load_builtin_providers, BuiltinModule};
pub use errors::Error;
pub use extractor::TargetProviders;
pub use provider::{provider_fields, register_provider, TypeId, UnpackProvider};

#[cfg(test)]
mod tests;
