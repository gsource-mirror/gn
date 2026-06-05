//  Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ffi/value.h"

#include <new>
#include <string>

#include "gn/ffi/bridge.h"
#include "gn/ffi/slice.h"

namespace rust {
ValueType cxx_to_rust(Value::Type t) {
  return static_cast<ValueType>(t);
}
}  // namespace rust

size_t ValueSize() {
  return sizeof(Value);
}

void SetValueNone(Value& self, const ParseNode* origin) {
  new (&self) Value(origin, Value::NONE);
}

void SetValueBool(Value& self, const ParseNode* origin, bool b) {
  new (&self) Value(origin, b);
}

void SetValueInt(Value& self, const ParseNode* origin, int64_t i) {
  new (&self) Value(origin, i);
}

void SetValueString(Value& self, const ParseNode* origin, rust::Str s) {
  new (&self) Value(origin, std::string(s.data(), s.size()));
}

Any* SetValueList(Value& self, const ParseNode* origin, size_t size) {
  new (&self) Value(origin, Value::LIST);
  self.list_value().resize(size);
  return reinterpret_cast<Any*>(self.list_value().data());
}

void SetValueScope(Value& self,
                   const ParseNode* origin,
                   std::unique_ptr<Scope> scope) {
  new (&self) Value(origin, std::move(scope));
}

SliceAny GetValueList(const Value& self) {
  return AsSlice(self.list_value());
}

std::unique_ptr<Value> NewValueForTesting() {
  return std::make_unique<Value>();
}
