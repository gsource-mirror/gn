// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/phony.h"

#include "gn/filesystem_utils.h"
#include "gn/ninja_target_writer.h"
#include "gn/path_output.h"
#include "gn/test_with_scheduler.h"
#include "gn/test_with_scope.h"
#include "util/test/test.h"

class PhonyTest : public TestWithScheduler {};

TEST_F(PhonyTest, NoPhonyRequired) {
  TestWithScope setup;
  setup.build_settings()->set_no_stamp_files(true);
  TestTarget target(setup, "//foo:bar", Target::GROUP);

  std::optional<OutputFile> empty_phony = target.add_phony({}, {}, ".suffix1");
  EXPECT_FALSE(empty_phony.has_value());

  OutputFile file(setup.build_settings(), SourceFile("//foo/a"));
  std::optional<OutputFile> direct_phony =
      target.add_phony({file}, {}, ".suffix2");
  ASSERT_TRUE(direct_phony.has_value());
  EXPECT_EQ(*direct_phony, file);

  std::optional<OutputFile> transitive_phony =
      target.add_phony({}, {direct_phony}, ".suffix3");
  ASSERT_TRUE(transitive_phony.has_value());
  EXPECT_EQ(*transitive_phony, file);

  Err err;
  ASSERT_TRUE(target.OnResolved(&err));

  ResolvedTargetData resolved;
  std::string output = NinjaTargetWriter::RunAndWriteFile(&target, &resolved);
  EXPECT_TRUE(output.empty());
}

TEST_F(PhonyTest, Multiple) {
  TestWithScope setup;
  setup.build_settings()->set_no_stamp_files(true);
  TestTarget target(setup, "//foo:bar", Target::GROUP);

  // 1. Multiple direct files (phony_a)
  OutputFile file1(setup.build_settings(), SourceFile("//foo/a"));
  OutputFile file2(setup.build_settings(), SourceFile("//foo/b"));
  OutputFile file3(setup.build_settings(), SourceFile("//foo/c"));
  std::optional<OutputFile> phony_a =
      target.add_phony({file2, file1}, {}, ".suffix1");

  ASSERT_TRUE(phony_a.has_value());
  EXPECT_EQ("phony/foo/bar.suffix1", phony_a->value());

  // 2. Transitive phony on the same target (phony_b depends on phony_a)
  std::optional<OutputFile> phony_b =
      target.add_phony({file2, file3}, {phony_a}, ".suffix2");

  ASSERT_TRUE(phony_b.has_value());
  EXPECT_EQ("phony/foo/bar.suffix2", phony_b->value());

  Err err;
  ASSERT_TRUE(target.OnResolved(&err));

  ResolvedTargetData resolved;
  std::string output = NinjaTargetWriter::RunAndWriteFile(&target, &resolved);

  std::string expected =
      "build phony/foo/bar.suffix1: phony ../../foo/a ../../foo/b\n"
      "build phony/foo/bar.suffix2: phony ../../foo/b ../../foo/c "
      "phony/foo/bar.suffix1\n";
  EXPECT_EQ(expected, output);
}
