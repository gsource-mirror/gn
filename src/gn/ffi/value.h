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
// Note: Rust may call these functions with uninitialized Value objects.
// Thus, in-place construction should be used without first destroying the
// object.
void SetValueNone(Value& self, const ParseNode* origin);
void SetValueBool(Value& self, const ParseNode* origin, bool b);
void SetValueInt(Value& self, const ParseNode* origin, int64_t i);
void SetValueString(Value& self, const ParseNode* origin, rust::Str s);
struct Any;
Any* SetValueList(Value& self, const ParseNode* origin, size_t size);
void SetValueScope(Value& self,
                   const ParseNode* origin,
                   std::unique_ptr<Scope> scope);

SliceAny GetValueList(const Value& self);

// GN values are never created in starlark in production code.
// If a value may ever be returned, it will be passed as a mutable output
// parameter.
std::unique_ptr<Value> NewValueForTesting();

#endif  // TOOLS_GN_FFI_VALUE_H_
