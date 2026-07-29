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
                  const SourceDir& from,
                  std::vector<rust::Str> keys,
                  Scope& dest_scope,
                  ParseNodePtr parse_node,
                  Err& err) {
  session.load_values(rust::Str(label.data(), label.size()),
                      rust::Str(from.SourceWithNoTrailingSlash()),
                      rust::Slice<const rust::Str>(keys.data(), keys.size()),
                      dest_scope, *dest_scope.settings(), parse_node, err);

  return !err.has_error();
}
