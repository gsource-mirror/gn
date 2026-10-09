// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/check_includes_strict_exceptions.h"

#include <string>
#include <string_view>

#include "gn/input_file.h"
#include "gn/label.h"
#include "gn/source_dir.h"
#include "gn/source_file.h"
#include "util/test/test.h"

TEST(CheckIncludesStrictExceptionsTest, Parse) {
  auto parse = [](std::string_view contents) {
    InputFile input_file(SourceFile("//exceptions.txt"));
    input_file.SetContents(std::string(contents));
    return CheckIncludesStrictExceptions::Parse(input_file);
  };

  EXPECT_FAILURE(parse("//* -> //*"));
  EXPECT_FAILURE(parse("not_pattern => //*"));
  EXPECT_FAILURE(parse(":relative => //*"));
  EXPECT_FAILURE(parse("/absolute => //*"));
  EXPECT_FAILURE(parse("//:multi => \n//:line"));
  EXPECT_FAILURE(parse("//:with(//toolchain) => //:*"));

  std::string_view input =
      "# Empty line below\n"
      "\n"
      "\r\n"
      "//:includer => //:included\n";

  auto exceptions = parse(input);
  ASSERT_SUCCESS(exceptions);
  EXPECT_TRUE(exceptions->CanInclude(Label(SourceDir("//"), "includer"),
                                     Label(SourceDir("//"), "included")));
  EXPECT_FALSE(exceptions->CanInclude(Label(SourceDir("//"), "included"),
                                      Label(SourceDir("//"), "includer")));
}

TEST(CheckIncludesStrictExceptionsTest, CanInclude) {
  CheckIncludesStrictExceptions exceptions;

  exceptions.AddException(
      LabelPattern(LabelPattern::RECURSIVE_DIRECTORY,
                   SourceDir("//includer_recursive/"), "", Label()),
      LabelPattern(LabelPattern::DIRECTORY, SourceDir("//included_dir/"), "",
                   Label()));

  EXPECT_TRUE(exceptions.CanInclude(
      Label(SourceDir("//includer_recursive/sub/"), "target"),
      Label(SourceDir("//included_dir/"), "foo")));
  EXPECT_FALSE(
      exceptions.CanInclude(Label(SourceDir("//other_dir/"), "target"),
                            Label(SourceDir("//included_dir/"), "foo")));
  EXPECT_FALSE(exceptions.CanInclude(
      Label(SourceDir("//includer_recursive/sub/"), "target"),
      Label(SourceDir("//included_dir/sub/"), "foo")));

  exceptions.AddException(
      LabelPattern(LabelPattern::DIRECTORY, SourceDir("//includer_dir/"), "",
                   Label()),
      LabelPattern(LabelPattern::MATCH, SourceDir("//included/"), "target",
                   Label()));

  EXPECT_TRUE(exceptions.CanInclude(Label(SourceDir("//includer_dir/"), "foo"),
                                    Label(SourceDir("//included/"), "target")));
  EXPECT_FALSE(
      exceptions.CanInclude(Label(SourceDir("//includer_dir/sub/"), "foo"),
                            Label(SourceDir("//included/"), "target")));
  EXPECT_FALSE(
      exceptions.CanInclude(Label(SourceDir("//includer_dir/"), "foo"),
                            Label(SourceDir("//included/"), "other_target")));

  exceptions.AddException(
      LabelPattern(LabelPattern::MATCH, SourceDir("//includer/"), "target",
                   Label()),
      LabelPattern(LabelPattern::RECURSIVE_DIRECTORY,
                   SourceDir("//included_recursive/"), "", Label()));

  EXPECT_TRUE(exceptions.CanInclude(
      Label(SourceDir("//includer/"), "target"),
      Label(SourceDir("//included_recursive/sub/"), "foo")));
  EXPECT_FALSE(exceptions.CanInclude(
      Label(SourceDir("//includer/"), "other_target"),
      Label(SourceDir("//included_recursive/sub/"), "foo")));
  EXPECT_FALSE(exceptions.CanInclude(Label(SourceDir("//includer/"), "target"),
                                     Label(SourceDir("//other_dir/"), "foo")));
}
