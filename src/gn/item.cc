// Copyright (c) 2013 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/item.h"

#include "base/logging.h"
#include "gn/build_settings.h"
#include "gn/err.h"
#include "gn/ffi/bridge.h"
#include "gn/settings.h"

Item::Item(const Settings* settings,
           const Label& label,
           const SourceFileSet& build_dependency_files)
    : settings_(settings),
      label_(label),
      build_dependency_files_(build_dependency_files),
      defined_from_(nullptr) {}

Item::~Item() = default;

Config* Item::AsConfig() {
  return nullptr;
}
const Config* Item::AsConfig() const {
  return nullptr;
}
Pool* Item::AsPool() {
  return nullptr;
}
const Pool* Item::AsPool() const {
  return nullptr;
}
Target* Item::AsTarget() {
  return nullptr;
}
const Target* Item::AsTarget() const {
  return nullptr;
}
Toolchain* Item::AsToolchain() {
  return nullptr;
}
const Toolchain* Item::AsToolchain() const {
  return nullptr;
}

std::string Item::GetItemTypeName() const {
  if (AsConfig())
    return "config";
  if (AsTarget())
    return "target";
  if (AsToolchain())
    return "toolchain";
  if (AsPool())
    return "pool";
  NOTREACHED();
  return "this thing that I have no idea what it is";
}

bool Item::OnResolved(Err* err) {
  return true;
}

Err Item::RunStarlarkRule() {
  auto phony = ExecuteStarlarkRule();
  // Builtin types can't fail to run starlark.
  DCHECK(!phony.has_error());
  DCHECK(phony->empty());
  return Ok();
}

Result<std::string_view> Item::ExecuteStarlarkRule() {
  const auto& session = settings()->build_settings()->starlark_session();
  const RustTarget& rust = rust_target(session);

  Err err;
  auto phony = rust.execute_rule_impl(session, err);
  if (err.has_error()) {
    return err;
  }
  return std::string_view(phony);
}
