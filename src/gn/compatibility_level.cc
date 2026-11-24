// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/compatibility_level.h"

#include <algorithm>
#include <string>

#include "base/strings/string_number_conversions.h"
#include "gn/value.h"

// static
Result<CompatibilityLevel> CompatibilityLevel::Parse(const Value* value) {
  auto make_err = [value](const std::string& msg, const std::string& help) {
    return value ? Err(*value, msg, help) : Err(Location(), msg, help);
  };

  CompatibilityLevel level;
  if (value) {
    RETURN_IF_ERROR(value->VerifyTypeIs(Value::INTEGER));
    level = CompatibilityLevel(value->int_value());
  }

  if (level.supports(CompatibilityLevel::kNext)) {
    return make_err(
        "Unsupported compatibility_level " + base::Int64ToString(level.level_) +
            ".",
        "This version of GN only supports compatibility levels up to " +
            base::Int64ToString(CompatibilityLevel::kCurrent) +
            ". Please update GN.");
  }

  if (!level.supports(CompatibilityLevel::kMin)) {
    std::string help = "The minimum supported compatibility_level is " +
                       base::Int64ToString(CompatibilityLevel::kMin) +
                       " (currently requested " +
                       base::Int64ToString(level.level_) +
                       ").\nChanges required to upgrade:\n";
    for (auto i = std::max<int64_t>(level.level_, 0);
         i < CompatibilityLevel::kMin; ++i) {
      help += "  Level " + base::Int64ToString(i) + " -> " +
              base::Int64ToString(i + 1) + ": " +
              std::string(kCompatibilityLevels[i]) + "\n";
    }
    return make_err("compatibility_level " + base::Int64ToString(level.level_) +
                        " is no longer supported.",
                    help);
  }

  return CompatibilityLevel(level);
}
