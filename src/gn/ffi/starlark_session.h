// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_STARLARK_SESSION_H_
#define TOOLS_GN_STARLARK_SESSION_H_

#include <string>
#include <string_view>

#include "gn/ffi/types.h"

class BuildSettings;

class StarlarkSession {
 public:
  StarlarkSession(const BuildSettings& build_settings);
  ~StarlarkSession();

  StarlarkSession(const StarlarkSession&) = delete;
  StarlarkSession& operator=(const StarlarkSession&) = delete;

  const rust::StarlarkModule* load(std::string_view path,
                                   const SourceDir& dir,
                                   const Scope& scope,
                                   const ParseNode* origin,
                                   Err& err) const;

  const rust::StarlarkSession& rust_session() const { return *value_; }

 private:
  rust::StarlarkSession* value_;
};

#endif  // TOOLS_GN_STARLARK_SESSION_H_
