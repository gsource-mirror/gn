// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_FFI_SESSION_H_
#define TOOLS_GN_FFI_SESSION_H_

#include <string_view>
#include <vector>

struct Session;
class Scope;
class ParseNode;
class Err;

bool session_load(const Session& session,
                  std::string_view label,
                  std::vector<std::string_view> keys,
                  Scope& dest_scope,
                  ParseNode* parse_node,
                  Err& err);

#endif  // TOOLS_GN_FFI_SESSION_H_