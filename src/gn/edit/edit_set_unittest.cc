// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/edit/edit_test_helper.h"

#include "util/test/test.h"

namespace commands {

TEST(EditSetTest, SetAttributeTypes) {
  // New bool attribute
  {
    auto result = RunEditCommand("set testonly true",
                                 "executable(\"foo\") {\n"
                                 "}\n");
    ASSERT_SUCCESS(result,
                   "executable(\"foo\") {\n"
                   "  testonly = true\n"
                   "}\n");
  }

  // Replacing existing attribute
  {
    auto result = RunEditCommand("set testonly true",
                                 "executable(\"foo\") {\n"
                                 "  testonly = false\n"
                                 "}\n");
    ASSERT_SUCCESS(result,
                   "executable(\"foo\") {\n"
                   "  testonly = true\n"
                   "}\n");
  }

  // String attribute
  {
    auto result = RunEditCommand("set label \"//foo:bar\"",
                                 "executable(\"foo\") {\n"
                                 "}\n");
    ASSERT_SUCCESS(result,
                   "executable(\"foo\") {\n"
                   "  label = \"//foo:bar\"\n"
                   "}\n");
  }

  // Int attribute
  {
    auto result = RunEditCommand("set assert_no_deps 42",
                                 "executable(\"foo\") {\n"
                                 "}\n");
    ASSERT_SUCCESS(result,
                   "executable(\"foo\") {\n"
                   "  assert_no_deps = 42\n"
                   "}\n");
  }
}

TEST(EditSetTest, HandleConditionalAssignment) {
  auto result = RunEditCommand("set testonly true",
                               "executable(\"foo\") {\n"
                               "  if (is_linux) {\n"
                               "    testonly = true\n"
                               "  }\n"
                               "}\n");
  ASSERT_SUCCESS(result,
                 "executable(\"foo\") {\n"
                 "  if (is_linux) {\n"
                 "    # TODO(gn edit): This is conditional, so double check "
                 "this is safe to remove\n"
                 "    testonly = true\n"
                 "  }\n"
                 "  testonly = true\n"
                 "}\n");
}

TEST(EditSetTest, HandleModificationAssignment) {
  auto result = RunEditCommand("set deps \"[ \\\"//baz\\\" ]\"",
                               "executable(\"foo\") {\n"
                               "  deps += [ \"//bar\" ]\n"
                               "}\n");
  ASSERT_SUCCESS(result,
                 "executable(\"foo\") {\n"
                 "  # TODO(gn edit): This is a modification, so double check "
                 "this is safe to\n"
                 "  # remove\n"
                 "  deps += [ \"//bar\" ]\n"
                 "  deps = [ \"//baz\" ]\n"
                 "}\n");
}

TEST(EditSetTest, MultipleTargetsSubset) {
  auto result = RunEditCommand("set testonly true", {"//:foo"},
                               "executable(\"foo\") {\n"
                               "  testonly = false\n"
                               "}\n"
                               "executable(\"bar\") {\n"
                               "  testonly = false\n"
                               "}\n");
  ASSERT_SUCCESS(result,
                 "executable(\"foo\") {\n"
                 "  testonly = true\n"
                 "}\n"
                 "executable(\"bar\") {\n"
                 "  testonly = false\n"
                 "}\n");
}

TEST(EditSetTest, PatternNeverMatched) {
  auto result = RunEditCommand("set testonly true", {"//:nonexistent"},
                               "executable(\"foo\") {\n"
                               "}\n");
  EXPECT_FAILURE(result, "Target(s) not found: //:nonexistent");
}

}  // namespace commands
