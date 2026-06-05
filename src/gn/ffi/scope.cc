// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ffi/scope.h"
#include "gn/ffi/bridge.h"
#include "gn/ffi/slice.h"
#include "gn/scope.h"
#include "gn/value.h"

SliceAny NewScope(const Scope& parent_scope,
                  rust::Slice<const rust::Str> keys,
                  std::unique_ptr<Scope>& out_scope) {
  auto new_scope = std::make_unique<Scope>(&parent_scope);
  new_scope->set_source_dir(parent_scope.GetSourceDir());
  new_scope->DetachFromContaining();

  std::vector<Value*> placeholders;
  placeholders.reserve(keys.size());
  for (const auto& key : keys) {
    std::string_view key_sv(key.data(), key.size());
    Value* val = new_scope->SetValue(key_sv, Value(), nullptr);
    placeholders.push_back(val);
  }

  out_scope = std::move(new_scope);
  return IntoSlice(std::move(placeholders));
}

// Unlike regular GN scoping rules, this does not extract from variables defined
// in outer scopes. Allowing variables defined in outer scopes would introduce
// spooky action at a distance, which, in the starlark side of things, is far
// better managed by just using macros wrapping rules that set sensible
// defaults.
SliceAny GetScopeItems(const Scope& scope) {
  Scope::KeyValueMap scope_values;
  scope.GetCurrentScopeValues(&scope_values);

  std::vector<KeyValue> vec;
  vec.reserve(scope_values.size());
  for (const auto& pair : scope_values) {
    vec.push_back(
        KeyValue{rust::Str(pair.first.data(), pair.first.size()), pair.second});
  }
  return IntoSlice(std::move(vec));
}
