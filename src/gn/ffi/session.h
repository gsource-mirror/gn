// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_FFI_SESSION_H_
#define TOOLS_GN_FFI_SESSION_H_

#include <string_view>
#include <vector>

#include "cxx.h"

struct Session;
class Scope;
class ParseNode;
class Err;
class SourceDir;

struct ParseNodePtr;

bool session_load(const Session& session,
                  std::string_view label,
                  const SourceDir& from,
                  std::vector<rust::Str> keys,
                  Scope& dest_scope,
                  ParseNodePtr parse_node,
                  Err& err);

#endif  // TOOLS_GN_FFI_SESSION_H_