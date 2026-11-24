// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_COMPATIBILITY_LEVEL_H_
#define TOOLS_GN_COMPATIBILITY_LEVEL_H_

#include <stdint.h>

#include <array>
#include <cstddef>
#include <string_view>

#include "gn/err.h"

class Value;

// Represents a GN backwards-compatibility level configured via `.gn`.
//
// Process for making backwards-incompatible changes:
// - File a tracking bug at https://gn.g-issues.chromium.org/issues/new for
//   deprecating the old compatibility level, and use bugjuggler to assign it
//   to yourself after 1 month has passed.
// - Add a new enum value (e.g. `kNewLevel`) to `CompatibilityLevel::Level`
//   immediately before `kNext`.
// - Gate the backwards-incompatible behavior on
//   `build_settings->supports(CompatibilityLevel::kNewLevel)`, keeping the
//   previous behavior for older levels.
// - Add an entry to `kCompatibilityLevels` describing the changes from the
//   previous level (including the tracking bug URL).
//
// Process for removing old compatibility levels / flags (after 1 month):
// - Check the tracking bug for any requests to extend the deprecation deadline.
// - Delete the oldest enumerator(s) from `CompatibilityLevel::Level` (must be
//   removed in order).
// - Add the number of deleted items to `kLastUnsupported`.
// - Replace `build_settings->supports(kRemovedFlag)` with `true` and simplify
//   code paths.
class CompatibilityLevel {
 public:
  enum Level : int64_t {
    // The minimum supported level - 1.
    // This must be incremented when deleting enum entries, and enum entries
    // must be deleted in order.
    kLastUnsupported = -1,

    kInitial,

    kNext,
  };

  static constexpr Level kMin =
      static_cast<Level>(static_cast<int64_t>(Level::kLastUnsupported) + 1);
  static constexpr Level kCurrent =
      static_cast<Level>(static_cast<int64_t>(Level::kNext) - 1);

  constexpr CompatibilityLevel() = default;
  constexpr CompatibilityLevel(Level level) : level_(level) {}

  // Extracts and validates `compatibility_level` against `[kMin, kCurrent]`.
  static Result<CompatibilityLevel> Parse(const Value* value);

  constexpr bool supports(const CompatibilityLevel& other) const {
    return level_ >= other.level_;
  }

 private:
  constexpr explicit CompatibilityLevel(int64_t level)
      : level_(static_cast<Level>(level)) {}

  // By defaulting to kCurrent, we ensure that any project that does not
  // explicitly specify `compatibility_level` is pinned to the latest.
  Level level_ = kCurrent;
};

// Descriptions of breaking changes between adjacent compatibility levels.
// Entry `i` describes the changes between compatibility level `i` and `i + 1`.
inline constexpr std::array<std::string_view,
                            static_cast<size_t>(CompatibilityLevel::kCurrent)>
    kCompatibilityLevels = {};

#endif  // TOOLS_GN_COMPATIBILITY_LEVEL_H_
