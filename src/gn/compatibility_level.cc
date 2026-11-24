// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/compatibility_level.h"

#include <string>

#include "base/strings/string_number_conversions.h"
#include "gn/value.h"

// static
Result<CompatibilityLevel> CompatibilityLevel::Parse(const Value* value) {
  auto make_err = [value](const std::string& msg, const std::string& help) {
    return value ? Err(*value, msg, help) : Err(Location(), msg, help);
  };

  int64_t level = 0;
  if (value) {
    RETURN_IF_ERROR(value->VerifyTypeIs(Value::INTEGER));
    level = value->int_value();
  }

  if (level < 0) {
    return make_err("Invalid compatibility_level.",
                    "compatibility_level must be a non-negative integer.");
  }

  constexpr int64_t max_level = static_cast<int64_t>(kCurrent);
  if (level > max_level) {
    return make_err(
        "Unsupported compatibility_level " + base::Int64ToString(level) + ".",
        "This version of GN only supports compatibility levels up to " +
            base::Int64ToString(max_level) + ". Please update GN.");
  }

  constexpr int64_t min_level = static_cast<int64_t>(kMin);
  if (level < min_level) {
    std::string help =
        "The minimum supported compatibility_level is " +
        base::Int64ToString(min_level) + " (current maximum is " +
        base::Int64ToString(max_level) + ").\nChanges required to upgrade:\n";
    for (int64_t i = level; i < min_level; ++i) {
      help += "  Level " + base::Int64ToString(i) + " -> " +
              base::Int64ToString(i + 1) + ": " +
              std::string(kCompatibilityLevels[i]) + "\n";
    }
    return make_err("compatibility_level " + base::Int64ToString(level) +
                        " is no longer supported.",
                    help);
  }

  return CompatibilityLevel(level);
}
