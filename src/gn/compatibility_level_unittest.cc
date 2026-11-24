// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/compatibility_level.h"

#include <stdint.h>

#include <string>

#include "gn/value.h"
#include "util/test/test.h"

TEST(CompatibilityLevelTest, ParseDefault) {
  Result<CompatibilityLevel> result = CompatibilityLevel::Parse(nullptr);
  ASSERT_TRUE(result.has_value());
  EXPECT_TRUE(result->supports(CompatibilityLevel::kCurrent));
  EXPECT_FALSE(result->supports(CompatibilityLevel::kNext));
}

TEST(CompatibilityLevelTest, ParseLastUnsupported) {
  Value value(nullptr, CompatibilityLevel::kLastUnsupported);
  Result<CompatibilityLevel> result = CompatibilityLevel::Parse(&value);
  ASSERT_TRUE(result.has_error());
  EXPECT_EQ("compatibility_level " +
                std::to_string(CompatibilityLevel::kLastUnsupported) +
                " is no longer supported.",
            result.message());
}

TEST(CompatibilityLevelTest, ParseMin) {
  Value value(nullptr, static_cast<int64_t>(CompatibilityLevel::kMin));
  Result<CompatibilityLevel> result = CompatibilityLevel::Parse(&value);
  ASSERT_TRUE(result.has_value());
  EXPECT_TRUE(result->supports(CompatibilityLevel::kMin));
  EXPECT_FALSE(
      result->supports(CompatibilityLevel(CompatibilityLevel::kMin + 1)));
}

TEST(CompatibilityLevelTest, ParseCurrent) {
  Value value(nullptr, static_cast<int64_t>(CompatibilityLevel::kCurrent));
  Result<CompatibilityLevel> result = CompatibilityLevel::Parse(&value);
  ASSERT_TRUE(result.has_value());
  EXPECT_TRUE(result->supports(CompatibilityLevel::kCurrent));
  EXPECT_FALSE(result->supports(CompatibilityLevel::kNext));
}

TEST(CompatibilityLevelTest, ParseNext) {
  Value value(nullptr, CompatibilityLevel::kNext);
  Result<CompatibilityLevel> result = CompatibilityLevel::Parse(&value);
  ASSERT_TRUE(result.has_error());
  EXPECT_EQ("Unsupported compatibility_level " +
                std::to_string(CompatibilityLevel::kNext) + ".",
            result.message());
}
