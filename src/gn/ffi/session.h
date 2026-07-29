// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_FFI_SESSION_H_
#define TOOLS_GN_FFI_SESSION_H_

#include <span>
#include <string_view>

#include "gn/ffi/bridge.h"
#include "gn/tokenizer.h"

struct Session;
class Scope;
class ParseNode;
class Err;

struct ParseNodePtr;

bool session_load(const Session& session,
                  const Value& label,
                  std::span<const Value> keys,
                  Scope& dest_scope,
                  ParseNodePtr parse_node,
                  Err& err);

#endif  // TOOLS_GN_FFI_SESSION_H_