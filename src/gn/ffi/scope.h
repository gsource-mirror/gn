// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_FFI_SCOPE_H_
#define TOOLS_GN_FFI_SCOPE_H_

#include <memory>
#include "cxx.h"

class Scope;
struct SliceAny;

SliceAny NewScope(const Scope& parent_scope,
                  rust::Slice<const rust::Str> keys,
                  std::unique_ptr<Scope>& out_scope);
SliceAny GetScopeItems(const Scope& scope);

#endif  // TOOLS_GN_FFI_SCOPE_H_
