// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ffi/starlark_value.h"

#include "gn/ffi/rust_api.h"
#include "gn/value.h"
#include "gn/input_file.h"
#include "gn/parse_tree.h"

#include "gn/err.h"
#include "gn/scope.h"

StarlarkValue::StarlarkValue(rust::OwnedFrozenValue* value) : value_(value) {}

StarlarkValue::StarlarkValue(const Value& value, const rust::StarlarkSession& loader)
    : StarlarkValue(rust::convert_starlark_value(value, loader)) {}

StarlarkValue::~StarlarkValue() {
  if (value_) {
    rust::free_starlark_value(*value_);
  }
}

StarlarkValue::StarlarkValue(const StarlarkValue& other)
    : value_(other.value_ ? rust::clone_starlark_value(*other.value_) : nullptr) {}

StarlarkValue& StarlarkValue::operator=(const StarlarkValue& other) {
  if (this != &other) {
    if (value_) {
      rust::free_starlark_value(*value_);
    }
    value_ = other.value_ ? rust::clone_starlark_value(*other.value_) : nullptr;
  }
  return *this;
}

StarlarkValue::StarlarkValue(StarlarkValue&& other) noexcept : value_(other.value_) {
  other.value_ = nullptr;
}

StarlarkValue& StarlarkValue::operator=(StarlarkValue&& other) noexcept {
  if (this != &other) {
    if (value_) {
      rust::free_starlark_value(*value_);
    }
    value_ = other.value_;
    other.value_ = nullptr;
  }
  return *this;
}

std::string StarlarkValue::pretty() const {
  std::string out;
  if (value_) {
    rust::pretty_starlark_value(*value_, out);
  }
  return out;
}

void StarlarkValue::call(const std::vector<StarlarkValue>& args,
                         const std::vector<KeyVal>& kwargs,
                         Value& result,
                         Scope& scope,
                         const ParseNode* origin,
                         Err& err) const {
  rust::call_starlark_value(*this, args.data(), args.size(), kwargs.data(), kwargs.size(), result, &scope, origin, err);
}

std::vector<KeyVal> collect_scope_to_kwargs(const Scope& scope, const rust::StarlarkSession& loader) {
  std::vector<KeyVal> kwargs;
  Scope::KeyValueMap values;
  scope.GetCurrentScopeValues(&values);
  for (const auto& pair : values) {
    kwargs.emplace_back(rust::Str(pair.first.data(), pair.first.size()), pair.second, loader);
  }
  return kwargs;
}
