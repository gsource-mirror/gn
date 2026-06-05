// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod action;
pub mod args;
pub mod errors;

pub use action::Action;
pub use args::{
    args_methods, args_test_methods, register_args_test_globals, Args, ArgsGen, ArgsGenFrozen,
    FrozenArgs,
};
pub use errors::Error;

#[cfg(test)]
mod tests;
