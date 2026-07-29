// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ffi/session.h"

#include "gn/ffi/bridge.h"
#include "gn/parse_tree.h"
#include "gn/scope.h"
#include "gn/value.h"

bool session_load(const Session& session,
                  std::string_view label,
                  std::vector<std::string_view> keys,
                  Scope& dest_scope,
                  ParseNode* parse_node,
                  Err& err) {
  std::vector<KeyValueMut> key_values;
  key_values.reserve(keys.size());

  for (size_t i = 0; i < keys.size(); ++i) {
    key_values.push_back(KeyValueMut{
        .key = rust::Str(keys[i].data(), keys[i].size()),
        .value = *dest_scope.SetValue(keys[i], Value(), parse_node),
    });
  }

  session.load_values(
      rust::Str(label.data(), label.size()),
      rust::Slice<KeyValueMut>(key_values.data(), key_values.size()), err);

  return !err.has_error();
}
