// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_FFI_TEST_WITH_SCOPE_H_
#define TOOLS_GN_FFI_TEST_WITH_SCOPE_H_

#include <memory>

#include "gn/test_with_scope.h"

inline std::unique_ptr<TestWithScope> NewTestWithScope() {
  return std::make_unique<TestWithScope>();
}

#endif  // TOOLS_GN_FFI_TEST_WITH_SCOPE_H_
