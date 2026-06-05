// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ffi/starlark_session.h"

#include "gn/build_settings.h"
#include "gn/filesystem_utils.h"
#include "gn/source_dir.h"
#include "gn/ffi/rust_api.h"

StarlarkSession::StarlarkSession(const BuildSettings& build_settings) {
  std::string abs_path = build_settings.root_path_utf8();
  std::string rel_path;
  if (!build_settings.build_dir().is_null()) {
    rel_path = RebasePath("//", build_settings.build_dir(), build_settings.root_path_utf8());
  } else {
    rel_path = ".";
  }
  value_ = rust::new_starlark_session(rust::Str(abs_path.data(), abs_path.size()),
                                      rust::Str(rel_path.data(), rel_path.size()));
}

StarlarkSession::~StarlarkSession() {
  rust::free_starlark_session(value_);
}

const rust::StarlarkModule* StarlarkSession::load(std::string_view path,
                                                 const SourceDir& dir,
                                                 const Scope& scope,
                                                 const ParseNode* origin,
                                                 Err& err) const {
  return rust::starlark_session_load(*value_,
                                     rust::Str(path.data(), path.size()),
                                     dir,
                                     const_cast<Scope*>(&scope),
                                     origin,
                                     err);
}
