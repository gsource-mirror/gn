// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_STARLARK_RUST_API_H_
#define TOOLS_GN_STARLARK_RUST_API_H_

#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "rust/cxx.h"
#include "gn/ffi/types.h"

// Contains rust APIs that can be called from C++

// Forward declare Rust extern "C" functions

namespace rust {
extern "C" {
StarlarkSession* new_starlark_session(rust::Str abs_path, rust::Str rel_path);
void free_starlark_session(StarlarkSession* val);

const rust::StarlarkModule* starlark_session_load(const StarlarkSession& loader,
                                                  rust::Str path,
                                                  const ::SourceDir& source_dir,
                                                  ::Scope* scope,
                                                  const ::ParseNode* origin,
                                                  ::Err& err);

rust::OwnedFrozenValue* clone_starlark_value(const rust::OwnedFrozenValue& val);
void pretty_starlark_value(const rust::OwnedFrozenValue& val, std::string& out);
void call_starlark_value(const ::StarlarkValue& val,
                         const ::StarlarkValue* args_ptr,
                         size_t args_len,
                         const ::KeyVal* kwargs_ptr,
                         size_t kwargs_len,
                         ::Value& result,
                         ::Scope* scope,
                         const ::ParseNode* origin,
                         ::Err& err);
rust::OwnedFrozenValue* convert_starlark_value(const ::Value& value,
                                               const StarlarkSession& loader);
rust::OwnedFrozenValue* convert_target(const ::Target& target,
                                       const rust::OwnedFrozenValue& rule,
                                       const StarlarkSession& loader);
rust::RustTarget* new_native_gn_target(const StarlarkSession& session, ::Target* target);
void free_starlark_value(rust::OwnedFrozenValue& val);
bool run_target_rule_implementation(rust::RustTarget* starlark_target,
                                    ::Scope* scope,
                                    const StarlarkSession& session,
                                    ::Err& err);
void get_custom_ninja(const ::Target& target, const StarlarkSession& session, std::string& out);
rust::Str get_extra_input(const StarlarkSession& session, rust::RustTarget* starlark_target);
void value_from_module(const rust::StarlarkModule& module,
                       rust::Str name,
                       Value* out,
                       ::Scope* scope,
                       const ::ParseNode* origin,
                       ::Err& err);
}
}  // namespace rust

#endif  // TOOLS_GN_STARLARK_RUST_API_H_
