// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <stddef.h>
#include <vector>

#include "rust/cxx.h"
#include "gn/ffi/types.h"
#include "gn/output_file.h"
template <typename T, typename U>
struct Pair {
  T first;
  U second;
};

#include "gn/err.h"
#include "gn/functions.h"
#include "gn/label.h"
#include "gn/parse_tree.h"
#include "gn/scope.h"
#include "gn/settings.h"
#include "gn/source_dir.h"
#include "gn/build_settings.h"
#include "gn/ffi/starlark_session.h"
#include "gn/ffi/starlark_value.h"
#include "gn/target.h"
#include "gn/target_generator.h"
#include "gn/value.h"
#include "gn/filesystem_utils.h"
#include "gn/string_atom.h"

extern "C" {

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

rust::String GetErrorMessage(const Err& err) {
  std::string err_msg = err.message();
  if (!err.help_text().empty()) {
    err_msg += "\n\n" + err.help_text();
  }
  return rust::String(err_msg);
}

bool ErrHasError(const Err& err) {
  return err.has_error();
}

void ResizeListValue(Value& val, size_t size) {
  val.list_value().resize(size);
}

void SetListValueAt(Value& val, size_t index, const Value& item) {
  val.list_value()[index] = item;
}

const Settings* GetSettingsFromScope(const Scope* scope) {
  return scope->settings();
}

const Label& GetToolchainLabelFromSettings(const Settings& settings) {
  return settings.toolchain_label();
}

const Label& GetDefaultToolchainLabel(const Target& target) {
  return target.settings()->default_toolchain_label();
}

const Label& GetTargetLabel(const Target& target) {
  return target.label();
}

void InitializeTargetScope(Value& val, Scope* parent_scope) {
  auto new_scope = std::make_unique<Scope>(parent_scope);
  val = Value(val.origin(), std::move(new_scope));
}

void SetSourceDir(Scope* scope, rust::Str dir) {
  scope->set_source_dir(SourceDir(std::string_view(dir.data(), dir.size())));
}

void InitializeRecordScope(Value& val, Scope* parent_scope) {
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

rust::Str GetSourceDirValue(const SourceDir& dir) {
  const std::string& val = dir.value();
  return rust::Str(val.data(), val.size());
}

bool IsActionTarget(const Target& target) {
  return target.output_type() == Target::ACTION ||
         target.output_type() == Target::ACTION_FOREACH;
}

rust::Str GetOutputFilePath(const OutputFile& file) {
  std::string_view v = file.value();
  return rust::Str(v.data(), v.size());
}

rust::String GetTargetOutputDir(const Target& target) {
  std::string path(GetBuildDirForTargetAsOutputFile(&target, BuildDirType::OBJ).value());
  return rust::String(path);
}

const Target* GetResolvedDependency(const Target& target,
                                    rust::Str pkg,
                                    rust::Str name) {
  std::string pkg_str(pkg.data(), pkg.size());
  std::string name_str(name.data(), name.size());
  if (!pkg_str.empty() && pkg_str.back() != '/') {
    pkg_str += '/';
  }

  auto matches = [&](const Label& label) {
    return label.name() == name_str && label.dir().value() == pkg_str;
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

size_t GetTargetDeps(const Target& target, const Target** out_deps, size_t max_count) {
  auto deps = target.private_deps();
  if (out_deps) {
    size_t count = std::min(deps.size(), max_count);
    for (size_t i = 0; i < count; ++i) {
      out_deps[i] = deps[i].ptr;
    }
  }
  return deps.size();
}

size_t GetTargetPublicDeps(const Target& target, const Target** out_deps, size_t max_count) {
  auto deps = target.public_deps();
  if (out_deps) {
    size_t count = std::min(deps.size(), max_count);
    for (size_t i = 0; i < count; ++i) {
      out_deps[i] = deps[i].ptr;
    }
  }
  return deps.size();
}

size_t GetTargetConfigs(const Target& target, const Label** out_configs, size_t max_count) {
  auto configs = target.configs();
  if (out_configs) {
    size_t count = std::min(configs.size(), max_count);
    for (size_t i = 0; i < count; ++i) {
      out_configs[i] = &configs[i].label;
    }
  }
  return configs.size();
}

size_t GetTargetPublicConfigs(const Target& target, const Label** out_configs, size_t max_count) {
  auto configs = target.public_configs();
  if (out_configs) {
    size_t count = std::min(configs.size(), max_count);
    for (size_t i = 0; i < count; ++i) {
      out_configs[i] = &configs[i].label;
    }
  }
  return configs.size();
}

size_t GetTargetPublicSources(const Target& target, rust::Str* out_sources, size_t max_count) {
  const auto& settings = target.settings();
  std::vector<rust::Str> sources;
  if (target.all_headers_public()) {
    for (const auto& f : target.sources()) {
      if (f.GetType() == SourceFile::SOURCE_H) {
        StringAtom output_file_owned = OutputFile(settings->build_settings(), f).value();
        std::string_view s = output_file_owned;
        sources.push_back(rust::Str(s.data(), s.size()));
      }
    }
  } else {
    for (const auto& f : target.public_headers()) {
      StringAtom output_file_owned = OutputFile(settings->build_settings(), f).value();
      std::string_view s = output_file_owned;
      sources.push_back(rust::Str(s.data(), s.size()));
    }
  }
  if (out_sources) {
    size_t count = std::min(sources.size(), max_count);
    for (size_t i = 0; i < count; ++i) {
      out_sources[i] = sources[i];
    }
  }
  return sources.size();
}

size_t GetTargetPrivateSources(const Target& target, rust::Str* out_sources, size_t max_count) {
  const auto& settings = target.settings();
  std::vector<rust::Str> sources;
  for (const auto& f : target.sources()) {
    if (!target.all_headers_public() || f.GetType() != SourceFile::SOURCE_H) {
      StringAtom output_file_owned = OutputFile(settings->build_settings(), f).value();
      std::string_view s = output_file_owned;
      sources.push_back(rust::Str(s.data(), s.size()));
    }
  }
  if (out_sources) {
    size_t count = std::min(sources.size(), max_count);
    for (size_t i = 0; i < count; ++i) {
      out_sources[i] = sources[i];
    }
  }
  return sources.size();
}

rust::Slice<const OutputFile> GetTargetOutputFiles(const Target& target) {
  const auto& outputs = target.computed_outputs();
  return rust::Slice<const OutputFile>(outputs.data(), outputs.size());
}
size_t CollectScopeToKwargs(const Scope* scope, Pair<rust::Str, const Value*>* out_kwargs, size_t max_count) {
  if (!scope) return 0;
  Scope::KeyValueMap scope_values;
  scope->GetCurrentScopeValues(&scope_values);
  if (out_kwargs) {
    size_t count = std::min(scope_values.size(), max_count);
    size_t i = 0;
    for (const auto& pair : scope_values) {
      if (i >= count) break;
      const Value* stable_val = scope->GetValue(pair.first);
      out_kwargs[i] = Pair<rust::Str, const Value*>{rust::Str(pair.first.data(), pair.first.size()), stable_val};
      i++;
    }
  }
  return scope_values.size();
}

size_t CollectValueToKwargs(const Value* value, Pair<rust::Str, const Value*>* out_kwargs, size_t max_count) {
  return CollectScopeToKwargs(value->scope_value(), out_kwargs, max_count);
}

const rust::OwnedFrozenValue* GetStarlarkValue(const Value& val) {
  return &val.starlark_value().to_rust();
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

Value* GetListValueAt(Value& val, size_t index) {
  return &val.list_value()[index];
}

void SetNoneValue(Value& val, const ParseNode* origin) {
  val = Value(origin, Value::NONE);
}

void SetBoolValue(Value& val, const ParseNode* origin, bool b) {
  val = Value(origin, b);
}

void SetIntValue(Value& val, const ParseNode* origin, int64_t i) {
  val = Value(origin, i);
}

void SetStringValue(Value& val, const ParseNode* origin, rust::Str s) {
  val = Value(origin, std::string(s.data(), s.size()));
}

void SetListValue(Value& val, const ParseNode* origin) {
  val = Value(origin, Value::LIST);
}

void SetStarlarkValue(Value& val, const ParseNode* origin, rust::OwnedFrozenValue* rust_val) {
  val = Value(origin, StarlarkValue(rust_val));
}

Value* CreateValue() {
  return new Value();
}

void FreeValue(Value* val) {
  delete val;
}

Err* CreateErr() {
  return new Err();
}

void FreeErr(Err* err) {
  delete err;
}

rust::Str GetLabelDir(const Label& label) {
  return rust::Str(label.dir().value().data(), label.dir().value().size());
}

rust::Str GetLabelName(const Label& label) {
  return rust::Str(label.name().data(), label.name().size());
}

int32_t GetValueType(const Value& val) {
  return static_cast<int32_t>(val.type());
}

bool GetBoolValue(const Value& val) {
  return val.boolean_value();
}

int64_t GetIntValue(const Value& val) {
  return val.int_value();
}

rust::Str GetStringValue(const Value& val) {
  return rust::Str(val.string_value().data(), val.string_value().size());
}

size_t GetListValueLen(const Value& val) {
  return val.list_value().size();
}

const Value* GetListValueAtConst(const Value& val, size_t index) {
  return &val.list_value()[index];
}

const rust::OwnedFrozenValue* GetStarlarkValueInner(const StarlarkValue& val) {
  return &val.to_rust();
}

} // extern "C"
