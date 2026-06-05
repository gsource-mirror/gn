// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod expand;
pub mod args;
pub mod errors;
pub mod formatter;
pub mod unpack;
#[cfg(test)]
mod tests;
pub use args::{args_methods, Args, ArgsGen, ArgsGenFrozen, FrozenArgs};
pub use errors::Error;
pub use formatter::Formatter;
pub use unpack::ArgsSequence;
