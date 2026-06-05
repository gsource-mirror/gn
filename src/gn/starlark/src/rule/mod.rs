// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

mod rule;
mod globals;
mod run;

pub use rule::{RuleCallableGen, RuleCallable, FrozenRuleCallable};
pub use run::run_target_rule_implementation;
pub use globals::{
    register_rule, collect_files_from_attr,
    collect_files_from_resolved_target,
};

