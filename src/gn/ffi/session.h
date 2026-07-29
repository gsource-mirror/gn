// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_FFI_SESSION_H_
#define TOOLS_GN_FFI_SESSION_H_

#include <string_view>
#include <vector>

#include "cxx.h"
#include "gn/err.h"
#include "gn/ffi/bridge.h"
#include "gn/range_utils.h"
#include "gn/scope.h"
#include "gn/source_dir.h"

struct Session;
class Scope;
class ParseNode;
class Err;
class SourceDir;

struct ParseNodePtr;

bool session_load(const Session& session,
                  std::string_view label,
                  const SourceDir& from,
                  RangeOf<std::string_view> auto&& keys,
                  Scope& dest_scope,
                  ParseNodePtr parse_node,
                  Err& err) {
  auto keys_slice =
      to_vec(keys | std::views::transform(
                        [](std::string_view key) { return rust::Str(key); }));

  session.load_values(rust::Str(label),
                      rust::Str(from.SourceWithNoTrailingSlash()),
                      rust::Slice<const rust::Str>(keys_slice), dest_scope,
                      *dest_scope.settings(), parse_node, err);

  return !err.has_error();
}

#endif  // TOOLS_GN_FFI_SESSION_H_