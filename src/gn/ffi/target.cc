// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ffi/target.h"

#include <string_view>

#include "base/logging.h"
#include "gn/config.h"
#include "gn/ffi/bridge.h"
#include "gn/functions.h"
#include "gn/item.h"
#include "gn/label.h"
#include "gn/label_ptr.h"
#include "gn/parse_tree.h"
#include "gn/pool.h"
#include "gn/source_dir.h"
#include "gn/target.h"
#include "gn/target_generator.h"

uint8_t output_type_u8(const Item& item) {
  if (const Target* target = item.AsTarget()) {
    return static_cast<uint8_t>(target->output_type());
  }
  // Keep in sync with types::OutputType discriminants in Rust.
  if (item.AsConfig()) {
    return 16;
  }
  if (item.AsToolchain()) {
    return 17;
  }
  if (item.AsPool()) {
    return 18;
  }
  NOTREACHED();
  return 0;
}

Target* create_target(Scope& scope,
                      ParseNodePtr origin,
                      rust::Str name,
                      rust::Str output_type,
                      Err& err) {
  return TargetGenerator::GenerateTarget(
      &scope, origin.ptr ? origin.ptr->AsFunctionCall() : nullptr,
      std::string_view(name), std::string_view(output_type), &err);
}

void register_dependency(Target& target,
                         rust::Str package,
                         rust::Str name,
                         rust::Str toolchain_package,
                         rust::Str toolchain_name) {
  target.starlark_deps().emplace_back(
      Label(SourceDir(std::string_view(package)), std::string_view(name),
            SourceDir(std::string_view(toolchain_package)),
            std::string_view(toolchain_name)));
}

const RustTarget& Item::rust_target(const Session& session) const {
  const RustTarget* target = rust_target_.load(std::memory_order_acquire);
  if (target) {
    return *target;
  }
  // register_cxx_item calls set_rust_target, so we don't need to bother
  // caching it ourselves.
  return session.register_cxx_item(*this);
}

const Item& target_as_item(const Target& target) {
  return target;
}

const Target& label_target_pair_target(const LabelTargetPair& pair) {
  DCHECK(pair.ptr);
  return *pair.ptr;
}
