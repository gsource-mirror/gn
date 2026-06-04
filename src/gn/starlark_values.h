// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_STARLARK_VALUES_H_
#define TOOLS_GN_STARLARK_VALUES_H_

#include <stdint.h>
#include <string>
#include "rust/cxx.h"

class Value;
class ParseNode;
class Scope;

namespace starlark_ffi {

enum class ValueKind : uint8_t;
struct StarlarkOpaqueValue;
struct KeyValuePair;

void set_bool_value(Value& val, const ParseNode* origin, bool b);
void set_int_value(Value& val, const ParseNode* origin, int64_t n);
void set_string_value(Value& val, const ParseNode* origin, rust::Str s);
void set_list_value_size(Value& val, const ParseNode* origin, size_t size);
Value& get_list_index(Value& val, size_t index);
void set_starlark_value(Value& val, const ParseNode* origin, rust::Box<StarlarkOpaqueValue> func);

ValueKind get_value_kind(const Value& val);
bool get_bool_value(const Value& val);
int64_t get_int_value(const Value& val);
rust::Str get_string_value(const Value& val);
size_t get_list_size(const Value& val);
const Value& get_list_value_index(const Value& val, size_t index);
const StarlarkOpaqueValue& get_starlark_value(const Value& val);
void get_scope_values(const Value& val, rust::Vec<KeyValuePair>& out);

}  // namespace starlark_ffi

#endif  // TOOLS_GN_STARLARK_VALUES_H_
