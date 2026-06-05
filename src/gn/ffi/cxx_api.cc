// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ffi/cxx_api.h"

#include <stddef.h>
#include <vector>

#include "gn/err.h"
#include "gn/functions.h"
#include "gn/label.h"
#include "gn/parse_tree.h"
#include "gn/scope.h"
#include "gn/settings.h"
#include "gn/source_dir.h"
#include "gn/build_settings.h"
#include "gn/ffi/starlark_session.h"
#include "gn/ffi/rust_api.h"
#include "gn/target.h"
#include "gn/target_generator.h"
#include "gn/value.h"
#include "gn/filesystem_utils.h"



void AddStarlarkTargetDependency(
    Target& target,
    rust::Str dep_dir,
    rust::Str dep_name,
    rust::Str toolchain_dir,
    rust::Str toolchain_name
) {
  SourceDir dir(std::string_view(dep_dir.data(), dep_dir.size()));
  SourceDir tc_dir(std::string_view(toolchain_dir.data(), toolchain_dir.size()));
  Label dep(dir, std::string_view(dep_name.data(), dep_name.size()),
               tc_dir, std::string_view(toolchain_name.data(), toolchain_name.size()));
  target.starlark_deps().push_back(LabelTargetPair(dep));
}


void PopulateErr(Err& err, rust::Str message) {
  err = Err(Location(), std::string(message));
}

void PopulateErrWithLocation(Err& err, rust::Str message, const ParseNode* origin) {
  LocationRange range = origin ? origin->GetRange() : LocationRange();
  err = Err(range, std::string(message));
}

void PopulateErrWithHelp(Err& err, rust::Str message, rust::Str help, const ParseNode* origin) {
  LocationRange range = origin ? origin->GetRange() : LocationRange();
  err = Err(range, std::string(message), std::string(help));
}

const Settings* GetSettingsFromScope(const Scope* scope) {
  return scope->settings();
}

const Label& GetToolchainLabelFromSettings(const Settings& settings) {
  return settings.toolchain_label();
}

const Label& GetTargetLabel(const Target& target) {
  return target.label();
}
void ResizeListValue(Value& val, size_t size) {
  val.list_value().resize(size);
}

void SetListValueAt(Value& val, size_t index, const Value& item) {
  val.list_value()[index] = item;
}



std::vector<KeyVal> collect_scope_to_kwargs(const Scope& scope, const rust::StarlarkSession& session) {
  Scope::KeyValueMap scope_values;
  scope.GetCurrentScopeValues(&scope_values);
  std::vector<KeyVal> result;
  result.reserve(scope_values.size());
  for (const auto& pair : scope_values) {
    result.push_back(KeyVal{rust::Str(pair.first.data(), pair.first.size()), pair.second, session});
  }
  return result;
}

std::string GetErrorMessage(const Err& err) {
  std::string err_msg = err.message();
  if (!err.help_text().empty()) {
    err_msg += "\n\n" + err.help_text();
  }
  return err_msg;
}

std::vector<KeyVal> collect_value_to_kwargs(const Value& value, const rust::StarlarkSession& session) {
  return collect_scope_to_kwargs(*value.scope_value(), session);
}

void InitializeTargetScope(Value& val, Scope* parent_scope) {
  auto new_scope = std::make_unique<Scope>(parent_scope);
  val = Value(val.origin(), std::move(new_scope));
}

void InitializeRecordScope(Value& val, Scope* parent_scope) {
  // Don't inherit the scope from parent scope. GN treats nested scopes as
  // being able to be flattened, but that doesn't match starlark semantics.
  auto new_scope = std::make_unique<Scope>(parent_scope->settings());
  new_scope->set_source_dir(parent_scope->GetSourceDir());
  val = Value(val.origin(), std::move(new_scope));
}

Value* SetScopeValueAt(Value& scope_val, rust::Str key) {
  return scope_val.scope_value()->SetValue(
      std::string_view(key.data(), key.size()),
      Value(),
      scope_val.origin()
  );
}

Target* CreateTarget(rust::Str target_type,
                     rust::Str target_name,
                     const ParseNode* origin,
                     Value& kwargs_scope_val,
                     Err* err) {
  std::string type_str(target_type.data(), target_type.size());
  std::string name_str(target_name.data(), target_name.size());

  const FunctionCallNode* function_call =
      static_cast<const FunctionCallNode*>(origin);

  Scope* generator_scope = kwargs_scope_val.scope_value();

  const Scope* default_scope = generator_scope->GetTargetDefaults(type_str);
  if (default_scope) {
    Scope::MergeOptions merge_options;
    merge_options.skip_private_vars = true;
    if (!default_scope->NonRecursiveMergeTo(generator_scope, merge_options,
                                            function_call, "target defaults", err)) {
      return nullptr;
    }
  }

  TargetGenerator::DefineTarget(generator_scope, function_call,
                                name_str, type_str, err);

  if (err->has_error()) {
    return nullptr;
  }

  Scope::ItemVector* collector = generator_scope->GetItemCollector();
  if (!collector || collector->empty()) {
    *err = Err(function_call, "Can't define a target in this context.");
    return nullptr;
  }

  return collector->back().get()->AsTarget();
}

const rust::StarlarkSession* GetStarlarkSessionFromScope(const Scope* scope) {
  return &scope->settings()->build_settings()->starlark_session().rust_session();
}

const SourceDir* GetScopeSourceDir(const Scope* scope) {
  return &scope->GetSourceDir();
}

bool IsActionTarget(const Target& target) {
  return target.output_type() == Target::ACTION ||
         target.output_type() == Target::ACTION_FOREACH;
}

const std::vector<OutputFile>& GetTargetOutputFiles(const Target& target) {
  return target.computed_outputs();
}

const Target* GetResolvedDependency(const Target& target,
                                    const std::string& pkg,
                                    const std::string& name) {
  std::string normalized_pkg = pkg;
  if (!normalized_pkg.empty() && normalized_pkg.back() != '/') {
    normalized_pkg += '/';
  }

  auto matches = [&](const Label& label) {
    return label.name() == name && label.dir().value() == normalized_pkg;
  };

  for (const auto& pair : target.starlark_deps()) {
    if (matches(pair.label))
      return pair.ptr;
  }
  for (const auto& pair : target.private_deps()) {
    if (matches(pair.label))
      return pair.ptr;
  }
  for (const auto& pair : target.public_deps()) {
    if (matches(pair.label))
      return pair.ptr;
  }
  for (const auto& pair : target.data_deps()) {
    if (matches(pair.label))
      return pair.ptr;
  }
  return nullptr;
}

rust::Str GetOutputFilePath(const OutputFile& file) {
  std::string_view v = file.value();
  return rust::Str(v.data(), v.size());
}

std::string GetTargetOutputDir(const Target& target) {
  return std::string(GetBuildDirForTargetAsOutputFile(&target, BuildDirType::OBJ).value());
}

Label* GetLabelFromPtr(const LabelPtr& ptr) {
  return ptr.ptr;
}

template <typename T>
std::vector<LabelPtr> ToLabelVector(const T& vec) {
  std::vector<LabelPtr> result;
  for (const auto& pair : vec) {
    result.push_back({const_cast<Label*>(&pair.label)});
  }
  return result;
}

template <typename T>
std::vector<TargetPtr> ToTargetVector(const T& vec) {
  std::vector<TargetPtr> result;
  for (const auto& pair : vec) {
    result.push_back({pair.ptr});
  }
  return result;
}

std::vector<TargetPtr> GetTargetDeps(const Target& target) {
  return ToTargetVector(target.private_deps());
}

std::vector<TargetPtr> GetTargetPublicDeps(const Target& target) {
  return ToTargetVector(target.public_deps());
}

std::vector<LabelPtr> GetTargetConfigs(const Target& target) {
  return ToLabelVector(target.configs());
}

std::vector<LabelPtr> GetTargetPublicConfigs(const Target& target) {
  return ToLabelVector(target.public_configs());
}
std::vector<RustStrWrapper> GetTargetPublicSources(const Target& target) {
  std::vector<RustStrWrapper> result;
  if (target.all_headers_public()) {
    for (const auto& f : target.sources()) {
      if (f.GetType() == SourceFile::SOURCE_H) {
        StringAtom output_file_owned =
            OutputFile(target.settings()->build_settings(), f).value();
        std::string_view s = output_file_owned;
        result.push_back(RustStrWrapper{s.data(), s.size()});
      }
    }
  } else {
    for (const auto& f : target.public_headers()) {
      StringAtom output_file_owned =
          OutputFile(target.settings()->build_settings(), f).value();
      std::string_view s = output_file_owned;
      result.push_back(RustStrWrapper{s.data(), s.size()});
    }
  }
  return result;
}

std::vector<RustStrWrapper> GetTargetPrivateSources(const Target& target) {
  std::vector<RustStrWrapper> result;
  for (const auto& f : target.sources()) {
    if (!target.all_headers_public() || f.GetType() != SourceFile::SOURCE_H) {
      StringAtom output_file_owned =
          OutputFile(target.settings()->build_settings(), f).value();
      std::string_view s = output_file_owned;
      result.push_back(RustStrWrapper{s.data(), s.size()});
    }
  }
  return result;
}

const rust::StarlarkSession* GetStarlarkSessionFromTarget(const Target& target) {
  return &target.settings()->build_settings()->starlark_session().rust_session();
}

const Label& GetTargetToolchainLabel(const Target& target) {
  return target.settings()->toolchain_label();
}

void SetTargetStarlarkTarget(Target* target, rust::RustTarget* starlark_target) {
  target->set_starlark_target(starlark_target);
}

rust::RustTarget* GetTargetStarlarkTarget(const Target* target) {
  return target->starlark_target();
}
