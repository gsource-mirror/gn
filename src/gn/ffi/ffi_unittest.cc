// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <string_view>
#include "util/test/test.h"

#include "gn/ffi/bridge.h"

// Verify std::string_view matches the expected 16-byte size and 8-byte
// alignment on 64-bit target platforms.
static_assert(sizeof(std::string_view) == sizeof(RustFfiStringView),
              "std::string_view size mismatch");
static_assert(alignof(std::string_view) == alignof(RustFfiStringView),
              "std::string_view alignment mismatch");

TEST(FfiTest, StringViewLayoutTest) {
  auto s = std::string_view("hi");
  auto& layout = reinterpret_cast<const RustFfiStringView&>(s);
  EXPECT_EQ(layout.len, 2u);
  EXPECT_EQ(reinterpret_cast<const char*>(layout.ptr), s.data());
}
