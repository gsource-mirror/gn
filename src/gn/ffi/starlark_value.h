// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_STARLARK_VALUE_H_
#define TOOLS_GN_STARLARK_VALUE_H_

#include <string>
#include <vector>

#include "rust/cxx.h"

#include "gn/ffi/types.h"

class KeyVal;

// Similar to unique_ptr<OwnedFrozenValue>.
class StarlarkValue {
 public:
  StarlarkValue(rust::OwnedFrozenValue* value);
  StarlarkValue(const Value& value, const rust::StarlarkSession& loader);
  StarlarkValue(const StarlarkValue& other);
  StarlarkValue& operator=(const StarlarkValue& other);
  StarlarkValue(StarlarkValue&& other) noexcept;
  StarlarkValue& operator=(StarlarkValue&& other) noexcept;
  ~StarlarkValue();

  std::string pretty() const;
  const rust::OwnedFrozenValue& to_rust() const { return *value_; }

  void call(const std::vector<StarlarkValue>& args,
            const std::vector<KeyVal>& kwargs,
            Value& result,
            Scope& scope,
            const ParseNode* origin,
            Err& err) const;

 private:
  rust::OwnedFrozenValue* value_;
};

// A key-value pair suitable for passing through the FFI layer.
// Useful for dictionary-like structures such as scopes, records, structs, and
// kwargs
class KeyVal {
 public:
  KeyVal(rust::Str key, StarlarkValue value)
      : key_(key), value_(std::move(value)) {}
  KeyVal(rust::Str key, const Value& value, const rust::StarlarkSession& loader)
      : KeyVal(key, StarlarkValue(value, loader)) {}
  rust::Str key() const { return key_; }
  // Called by rust. C++ can't do anything with this
  const rust::OwnedFrozenValue& value() const { return value_.to_rust(); }

 private:
  rust::Str key_;
  StarlarkValue value_;
};

#endif  // TOOLS_GN_STARLARK_VALUE_H_
