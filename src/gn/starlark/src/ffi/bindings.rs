// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use autocxx::prelude::*;

#[allow(unused_imports)]

#[allow(unused_imports)]
use super::session::StarlarkSession;

include_cpp! {
    #include "gn/ffi/cxx_api.h"
    #include "gn/ffi/rust_api.h"
    #include "gn/ffi/starlark_value.h"
    #include "gn/label.h"
    #include "gn/output_file.h"
    #include "gn/source_dir.h"
    #include "gn/source_file.h"
    #include "gn/value.h"
    safety!(unsafe)
    generate!("Label")
    generate!("OutputFile")
    generate!("Settings")
    opaque!("Settings")
    generate!("Err")
    opaque!("Err")
    generate!("Scope")
    generate!("SourceDir")
    generate!("SourceFile")
    opaque!("Target")
    opaque!("Item")
    generate!("Value")
    generate!("StarlarkValue")
    generate!("ParseNode")
    opaque!("ParseNode")
    opaque!("Scope")
    block!("StringAtom")
    block!("Target::ModuleType")
    block!("base::FilePath")
    block!("base::Value")
    block!("Visibility")
    block!("Config")
    block!("Pool")
    block!("Toolchain")
    block!("std::unordered_map")
    block!("std::unordered_set")
    generate!("std::string_view")
    instantiable!("std::string_view")
    generate!("collect_value_to_kwargs")
    generate!("KeyVal")
    generate!("rust::StarlarkSession")
    generate!("rust::OwnedFrozenValue")
    generate!("rust::StarlarkModule")
    generate!("rust::RustTarget")
    generate_pod!("LabelPtr")
    generate_pod!("TargetPtr")
    generate_pod!("RustStrWrapper")
    generate!("GetLabelFromPtr")
    generate!("GetOutputFilePath")
    generate!("GetTargetOutputDir")
    generate!("AddStarlarkTargetDependency")
    generate!("PopulateErr")
    generate!("PopulateErrWithLocation")
    generate!("PopulateErrWithHelp")

    generate!("GetSettingsFromScope")
    generate!("GetToolchainLabelFromSettings")
    generate!("GetTargetLabel")
    generate!("GetStarlarkSessionFromScope")
    generate!("GetScopeSourceDir")
    generate!("IsActionTarget")
    generate!("GetTargetOutputFiles")
    generate!("GetResolvedDependency")
    generate!("GetTargetDeps")
    generate!("GetTargetPublicDeps")
    generate!("GetStarlarkSessionFromTarget")
    generate!("GetTargetToolchainLabel")
    generate!("SetTargetStarlarkTarget")
    generate!("GetTargetStarlarkTarget")
    generate!("GetTargetConfigs")
    generate!("GetTargetPublicConfigs")
    generate!("GetTargetPublicSources")
    generate!("GetTargetPrivateSources")
    generate!("ResizeListValue")
    generate!("SetListValueAt")
    generate!("InitializeTargetScope")
    generate!("InitializeRecordScope")
    generate!("SetScopeValueAt")
    generate!("CreateTarget")
    generate!("GetErrorMessage")
    generate!("NewTestWithScope")
    generate!("FreeTestWithScope")
    generate!("GetScopeFromTestWithScope")
    generate!("TestWithScope")
    opaque!("TestWithScope")
    generate!("rust::value_from_module")
}

pub use ffi::*;
