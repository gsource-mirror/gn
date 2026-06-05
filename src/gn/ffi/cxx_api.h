// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_STARLARK_CXX_API_H_
#define TOOLS_GN_STARLARK_CXX_API_H_

#include <stddef.h>
#include <string>
#include <vector>

#include "gn/ffi/starlark_value.h"
#include "gn/ffi/types.h"
#include "gn/output_file.h"
// Contains C++ APIs that can be called from rust

void AddStarlarkTargetDependency(
    Target& target,
    rust::Str dep_dir,
    rust::Str dep_name,
    rust::Str toolchain_dir,
    rust::Str toolchain_name
);
// TODO: remove this, we should always call with a location.
void PopulateErr(Err& err, rust::Str message);
void PopulateErrWithLocation(Err& err, rust::Str message, const ParseNode* origin);
void PopulateErrWithHelp(Err& err, rust::Str message, rust::Str help, const ParseNode* origin);
std::string GetErrorMessage(const Err& err);


void ResizeListValue(Value& val, size_t size);
void SetListValueAt(Value& val, size_t index, const Value& item);
void InitializeTargetScope(Value& val, Scope* parent_scope);
void InitializeRecordScope(Value& val, Scope* parent_scope);
Value* SetScopeValueAt(Value& scope_val, rust::Str key);
Target* CreateTarget(rust::Str target_type, rust::Str target_name, const ParseNode* origin, Value& kwargs_scope_val, Err* err);
const Settings* GetSettingsFromScope(const Scope* scope);
const Label& GetToolchainLabelFromSettings(const Settings& settings);
const Label& GetTargetLabel(const Target& target);
const rust::StarlarkSession* GetStarlarkSessionFromScope(const Scope* scope);
const SourceDir* GetScopeSourceDir(const Scope* scope);
TestWithScope* NewTestWithScope(rust::Str root_path, rust::Str build_dir);
void FreeTestWithScope(TestWithScope* setup);
Scope* GetScopeFromTestWithScope(TestWithScope& setup);
// Helper to extract values from a Scope.
std::vector<KeyVal> collect_scope_to_kwargs(const Scope& scope, const rust::StarlarkSession& session);

// Wrapper struct to allow returning vectors of pointers over FFI. cxx/autocxx
// does not support raw pointer types as elements in CxxVector.
struct LabelPtr {
  Label* ptr;
};

struct TargetPtr {
  const Target* ptr;
};

// C++ functions can't return Vector<rust::Str> since it's not a native C++
// type. So instead we return a vector of these wrappers.
struct RustStrWrapper {
  const char* ptr;
  size_t len;
};

Label* GetLabelFromPtr(const LabelPtr& ptr);
rust::Str GetOutputFilePath(const OutputFile& file);
std::string GetTargetOutputDir(const Target& target);

bool IsActionTarget(const Target& target);
const std::vector<OutputFile>& GetTargetOutputFiles(const Target& target);
const Target* GetResolvedDependency(const Target& target, const std::string& pkg, const std::string& name);
std::vector<TargetPtr> GetTargetDeps(const Target& target);
std::vector<TargetPtr> GetTargetPublicDeps(const Target& target);
std::vector<LabelPtr> GetTargetConfigs(const Target& target);
std::vector<LabelPtr> GetTargetPublicConfigs(const Target& target);
std::vector<RustStrWrapper> GetTargetPublicSources(const Target& target);
std::vector<RustStrWrapper> GetTargetPrivateSources(const Target& target);
std::vector<KeyVal> collect_value_to_kwargs(const Value& value, const rust::StarlarkSession& session);
const rust::StarlarkSession* GetStarlarkSessionFromTarget(const Target& target);
const Label& GetTargetToolchainLabel(const Target& target);
void SetTargetStarlarkTarget(Target* target, rust::RustTarget* starlark_target);
rust::RustTarget* GetTargetStarlarkTarget(const Target* target);

#endif  // TOOLS_GN_STARLARK_CXX_API_H_
