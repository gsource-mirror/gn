// Copyright 2016 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/pool.h"

#include <sstream>

#include "base/logging.h"
#include "gn/build_settings.h"
#include "gn/ffi/bridge.h"
#include "gn/settings.h"
#include "gn/target.h"

Pool::Pool(const Settings* settings,
           const Label& label,
           const SourceFileSet& build_dependency_files)
    : Item(settings, label, build_dependency_files) {
  FfiTarget::new_builtin(settings->build_settings()->starlark_session(),
                         claim_rust_target_storage(), Target::kPoolOutputType);
}

Pool::~Pool() = default;

Pool* Pool::AsPool() {
  return this;
}

const Pool* Pool::AsPool() const {
  return this;
}

std::string Pool::GetNinjaName(const Label& default_toolchain) const {
  bool include_toolchain = label().toolchain_dir() != default_toolchain.dir() ||
                           label().toolchain_name() != default_toolchain.name();
  return GetNinjaName(include_toolchain);
}

std::string Pool::GetNinjaName(bool include_toolchain) const {
  std::ostringstream buffer;
  if (include_toolchain) {
    DCHECK(label().toolchain_dir().is_source_absolute());
    std::string toolchain_dir = label().toolchain_dir().value();
    for (std::string::size_type i = 2; i < toolchain_dir.size(); ++i) {
      buffer << (toolchain_dir[i] == '/' ? '_' : toolchain_dir[i]);
    }
    buffer << label().toolchain_name() << "_";
  }

  DCHECK(label().dir().is_source_absolute());
  std::string label_dir = label().dir().value();
  for (std::string::size_type i = 2; i < label_dir.size(); ++i) {
    buffer << (label_dir[i] == '/' ? '_' : label_dir[i]);
  }
  buffer << label().name();
  return buffer.str();
}
