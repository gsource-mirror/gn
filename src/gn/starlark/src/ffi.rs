// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod bindings;
pub mod rust_types;
pub mod starlark_module;
pub mod session;
pub mod starlark_value;
pub mod string;
pub mod result;
#[cfg(test)]
pub mod test_with_scope;

pub use rust_types::{AsCxx, AsRust, ToRust, IntoCxx, IntoRust};
pub use result::handle_result;
pub use result::handle_result_with_message;
pub use session::StarlarkSession;
pub use starlark::values::OwnedFrozenValue;
pub(crate) use starlark_value::to_cxx_value;
pub use starlark::environment::FrozenModule;

pub use bindings::Value;
pub use bindings::Scope;
pub use bindings::ParseNode;
pub use bindings::Label;
pub use bindings::OutputFile;
pub use bindings::Settings;
pub use bindings::Err;
pub use bindings::SourceDir;
pub use bindings::SourceFile;
pub use bindings::Target;
pub use bindings::LabelPtr;
pub use bindings::TargetPtr;
pub use bindings::RustStrWrapper;
pub use bindings::GetLabelFromPtr;
pub use bindings::GetOutputFilePath;
pub use bindings::GetTargetOutputDir;

pub use bindings::AddStarlarkTargetDependency;
pub use bindings::PopulateErr;
pub use bindings::PopulateErrWithLocation;
pub use bindings::PopulateErrWithHelp;

pub use bindings::ResizeListValue;
pub use bindings::SetListValueAt;
pub use bindings::GetSettingsFromScope;
pub use bindings::GetToolchainLabelFromSettings;
pub use bindings::GetTargetLabel;
pub use bindings::GetStarlarkSessionFromScope;
pub use bindings::GetStarlarkSessionFromTarget;
pub use bindings::GetTargetToolchainLabel;
pub use bindings::SetTargetStarlarkTarget;
pub use bindings::GetTargetStarlarkTarget;
pub use bindings::GetScopeSourceDir;
pub use bindings::IsActionTarget;
pub use bindings::GetTargetOutputFiles;
pub use bindings::GetResolvedDependency;
pub use bindings::GetTargetDeps;
pub use bindings::GetTargetPublicDeps;
pub use bindings::GetTargetConfigs;
pub use bindings::GetTargetPublicConfigs;
pub use bindings::GetTargetPublicSources;
pub use bindings::GetTargetPrivateSources;
pub use bindings::CreateTarget;
pub use bindings::GetErrorMessage;
pub use bindings::InitializeTargetScope;
pub use bindings::InitializeRecordScope;
pub use bindings::SetScopeValueAt;
#[cfg(test)]
pub use test_with_scope::TestWithScope;

pub use autocxx::c_void;
