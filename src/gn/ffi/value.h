// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_FFI_VALUE_H_
#define TOOLS_GN_FFI_VALUE_H_

#include <memory>
#include "cxx.h"

#include "gn/value.h"

class Scope;
class ParseNode;
struct SliceAny;

enum class ValueType : uint8_t;

namespace rust {
ValueType cxx_to_rust(Value::Type t);
}

size_t ValueSize();
void SetValueNone(Value& self, const ParseNode* origin);
void SetValueBool(Value& self, const ParseNode* origin, bool b);
void SetValueInt(Value& self, const ParseNode* origin, int64_t i);
void SetValueString(Value& self, const ParseNode* origin, rust::Str s);
uint8_t* SetValueList(Value& self, const ParseNode* origin, size_t size);
void SetValueScope(Value& self,
                   const ParseNode* origin,
                   std::unique_ptr<Scope> scope);
SliceAny GetValueList(const Value& self);
std::unique_ptr<Value> NewValueForTesting();

#endif  // TOOLS_GN_FFI_VALUE_H_
