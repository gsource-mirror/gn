// Copyright (c) 2013 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/item.h"

#include "base/logging.h"
#include "gn/ffi/bridge.h"
#include "gn/settings.h"

Item::Item(const Settings* settings,
           const Label& label,
           const SourceFileSet& build_dependency_files)
    : settings_(settings),
      label_(label),
      build_dependency_files_(build_dependency_files),
      defined_from_(nullptr) {}

Item::~Item() {
  if (rust_target_initialized_) {
    drop_ffi_target(reinterpret_cast<FfiTarget*>(rust_target_storage_));
  }
}

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

bool Item::is_builtin() const {
  if (!rust_target_initialized_) {
    return true;
  }
  return rust_target().is_builtin();
}

const FfiTarget& Item::rust_target() const {
  DCHECK(rust_target_initialized_);
  return *reinterpret_cast<const FfiTarget*>(rust_target_storage_);
}

FfiTarget* Item::rust_target_mut() {
  DCHECK(!rust_target_initialized_);
  rust_target_initialized_ = true;
  return reinterpret_cast<FfiTarget*>(rust_target_storage_);
}
