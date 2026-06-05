// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

mod globals;
mod rule;
mod run;

pub use globals::register_rule;
pub use rule::{Rule, FrozenRule};
pub use run::run_target_rule_implementation;
