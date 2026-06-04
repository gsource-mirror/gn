// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/starlark_values.h"

#include <utility>
#include "gn/parse_tree.h"
#include "gn/scope.h"
#include "gn/value.h"
#include "gn_starlark/src/lib.rs.h"

namespace starlark_ffi {

void set_bool_value(Value& val, const ParseNode* origin, bool b) {
  val = Value(origin, b);
}

void set_int_value(Value& val, const ParseNode* origin, int64_t n) {
  val = Value(origin, n);
}

void set_string_value(Value& val, const ParseNode* origin, rust::Str s) {
  val = Value(origin, std::string(s));
}

void set_list_value_size(Value& val, const ParseNode* origin, size_t size) {
  val = Value(origin, Value::LIST);
  val.list_value().resize(size);
}

Value& get_list_index(Value& val, size_t index) {
  return val.list_value()[index];
}

void set_starlark_value(Value& val, const ParseNode* origin, rust::Box<StarlarkOpaqueValue> func) {
  val = Value(origin, std::move(func));
}

ValueKind get_value_kind(const Value& val) {
  switch (val.type()) {
    case Value::NONE: return ValueKind::NONE;
    case Value::BOOLEAN: return ValueKind::BOOLEAN;
    case Value::INTEGER: return ValueKind::INTEGER;
    case Value::STRING: return ValueKind::STRING;
    case Value::LIST: return ValueKind::LIST;
    case Value::SCOPE: return ValueKind::SCOPE;
    case Value::STARLARK_VALUE: return ValueKind::STARLARK_VALUE;
  }
  return ValueKind::NONE;
}

bool get_bool_value(const Value& val) {
  return val.boolean_value();
}

int64_t get_int_value(const Value& val) {
  return val.int_value();
}

rust::Str get_string_value(const Value& val) {
  return rust::Str(val.string_value());
}

size_t get_list_size(const Value& val) {
  return val.list_value().size();
}

const Value& get_list_value_index(const Value& val, size_t index) {
  return val.list_value()[index];
}

const StarlarkOpaqueValue& get_starlark_value(const Value& val) {
  return *val.starlark_value();
}

void get_scope_values(const Value& val, rust::Vec<KeyValuePair>& out) {
  for (const auto& pair : val.scope_value()->values()) {
    out.push_back(KeyValuePair{rust::Str(pair.first.data(), pair.first.size()), pair.second.value});
  }
}

}  // namespace starlark_ffi
