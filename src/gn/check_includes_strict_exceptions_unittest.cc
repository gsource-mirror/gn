// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/check_includes_strict_exceptions.h"

#include <memory>
#include <string>
#include <string_view>

#include "gn/label.h"
#include "gn/scope.h"
#include "gn/source_dir.h"
#include "gn/value.h"
#include "util/test/test.h"

TEST(CheckIncludesStrictExceptionsTest, FromValue) {
  auto parse = [](const Value& val) {
    return CheckIncludesStrictExceptions::FromValue(val, SourceDir("//"), "");
  };

  auto make_rule = [](std::string_view from, std::string_view to) {
    auto scope = std::make_unique<Scope>(static_cast<const Settings*>(nullptr));
    scope->SetValue("includer", Value(nullptr, std::string(from)), nullptr);
    if (!to.empty())
      scope->SetValue("included", Value(nullptr, std::string(to)), nullptr);
    Value v(nullptr, Value::SCOPE);
    v.SetScopeValue(std::move(scope));
    return v;
  };

  EXPECT_FAILURE(parse(Value(nullptr, "not_a_list")));

  Value bad_list(nullptr, Value::LIST);
  bad_list.list_value().push_back(Value(nullptr, "not_a_scope"));
  EXPECT_FAILURE(parse(bad_list));

  Value non_abs(nullptr, Value::LIST);
  non_abs.list_value().push_back(make_rule("not_abs:a", "//b:b"));
  EXPECT_FAILURE(parse(non_abs));

  // Specific target exception.
  Value specific_list(nullptr, Value::LIST);
  specific_list.list_value().push_back(make_rule("//:includer", "//:included"));
  auto specific_exceptions = parse(specific_list);
  ASSERT_SUCCESS(specific_exceptions);
  EXPECT_TRUE(specific_exceptions->HasException(
      Label(SourceDir("//"), "includer"), Label(SourceDir("//"), "included")));
  EXPECT_FALSE(specific_exceptions->HasException(
      Label(SourceDir("//"), "includer"), Label(SourceDir("//"), "other")));

  // Global exception (included omitted).
  Value global_list(nullptr, Value::LIST);
  global_list.list_value().push_back(make_rule("//:includer", ""));
  auto global_exceptions = parse(global_list);
  ASSERT_SUCCESS(global_exceptions);
  EXPECT_TRUE(
      global_exceptions->HasException(Label(SourceDir("//"), "includer"),
                                      Label(SourceDir("//any/"), "target")));
  EXPECT_FALSE(
      global_exceptions->HasException(Label(SourceDir("//other/"), "target"),
                                      Label(SourceDir("//any/"), "target")));
}

TEST(CheckIncludesStrictExceptionsTest, CanInclude) {
  CheckIncludesStrictExceptions exceptions;

  exceptions.AddLocalException(
      LabelPattern(LabelPattern::RECURSIVE_DIRECTORY,
                   SourceDir("//includer_recursive/"), "", Label()),
      Label(SourceDir("//included/"), "target"));

  EXPECT_TRUE(exceptions.HasException(
      Label(SourceDir("//includer_recursive/sub/"), "target"),
      Label(SourceDir("//included/"), "target")));
  EXPECT_FALSE(
      exceptions.HasException(Label(SourceDir("//other_dir/"), "target"),
                              Label(SourceDir("//included/"), "target")));
  EXPECT_FALSE(exceptions.HasException(
      Label(SourceDir("//includer_recursive/sub/"), "target"),
      Label(SourceDir("//included/"), "other_target")));

  exceptions.AddGlobalException(LabelPattern(
      LabelPattern::DIRECTORY, SourceDir("//includer_global/"), "", Label()));

  EXPECT_TRUE(
      exceptions.HasException(Label(SourceDir("//includer_global/"), "foo"),
                              Label(SourceDir("//any_dir/"), "any_target")));
  EXPECT_FALSE(
      exceptions.HasException(Label(SourceDir("//other_dir/"), "foo"),
                              Label(SourceDir("//any_dir/"), "any_target")));
}

TEST(CheckIncludesStrictExceptionsTest, Empty) {
  CheckIncludesStrictExceptions exceptions;
  EXPECT_TRUE(exceptions.empty());

  CheckIncludesStrictExceptions global_exceptions;
  global_exceptions.AddGlobalException(
      LabelPattern(LabelPattern::DIRECTORY, SourceDir("//a/"), "", Label()));
  EXPECT_FALSE(global_exceptions.empty());

  CheckIncludesStrictExceptions specific_exceptions;
  specific_exceptions.AddLocalException(
      LabelPattern(LabelPattern::MATCH, SourceDir("//a/"), "foo", Label()),
      Label(SourceDir("//b/"), "bar"));
  EXPECT_FALSE(specific_exceptions.empty());
}

TEST(CheckIncludesStrictExceptionsTest, HasGlobalAndLocalException) {
  CheckIncludesStrictExceptions exceptions;
  Label from(SourceDir("//from/"), "foo");
  Label to(SourceDir("//to/"), "bar");
  Label other(SourceDir("//other/"), "baz");

  EXPECT_FALSE(exceptions.HasGlobalException(from));
  EXPECT_FALSE(exceptions.HasLocalException(from, to));

  exceptions.AddGlobalException(
      LabelPattern(LabelPattern::DIRECTORY, SourceDir("//from/"), "", Label()));
  EXPECT_TRUE(exceptions.HasGlobalException(from));
  EXPECT_FALSE(exceptions.HasGlobalException(other));
  EXPECT_FALSE(exceptions.HasLocalException(from, to));

  exceptions.AddLocalException(
      LabelPattern(LabelPattern::MATCH, SourceDir("//from/"), "foo", Label()),
      to);
  EXPECT_TRUE(exceptions.HasLocalException(from, to));
  EXPECT_FALSE(exceptions.HasLocalException(from, other));
  EXPECT_FALSE(exceptions.HasLocalException(other, to));
}

TEST(LabelPatternsTest, ExactMatch) {
  LabelPatterns patterns;
  patterns.AddPattern(LabelPattern(LabelPattern::MATCH, SourceDir("//foo/bar/"),
                                   "baz", Label()));
  patterns.AddPattern(LabelPattern(LabelPattern::MATCH, SourceDir("//"),
                                   "root_target", Label()));

  EXPECT_TRUE(patterns.Matches(Label(SourceDir("//foo/bar/"), "baz")));
  EXPECT_TRUE(patterns.Matches(Label(SourceDir("//"), "root_target")));

  EXPECT_FALSE(patterns.Matches(Label(SourceDir("//foo/bar/"), "other")));
  EXPECT_FALSE(patterns.Matches(Label(SourceDir("//foo/"), "baz")));
  EXPECT_FALSE(patterns.Matches(Label(SourceDir("//foo/bar/sub/"), "baz")));
  EXPECT_FALSE(patterns.Matches(Label(SourceDir("//"), "other")));
}

TEST(LabelPatternsTest, DirectoryWildcard) {
  LabelPatterns patterns;
  patterns.AddPattern(LabelPattern(LabelPattern::DIRECTORY,
                                   SourceDir("//foo/bar/"), "", Label()));
  patterns.AddPattern(
      LabelPattern(LabelPattern::DIRECTORY, SourceDir("//"), "", Label()));

  EXPECT_TRUE(patterns.Matches(Label(SourceDir("//foo/bar/"), "any")));
  EXPECT_TRUE(patterns.Matches(Label(SourceDir("//foo/bar/"), "other")));
  EXPECT_TRUE(patterns.Matches(Label(SourceDir("//"), "root_any")));

  EXPECT_FALSE(patterns.Matches(Label(SourceDir("//foo/bar/sub/"), "any")));
  EXPECT_FALSE(patterns.Matches(Label(SourceDir("//foo/"), "any")));
}

TEST(LabelPatternsTest, RecursiveWildcard) {
  LabelPatterns patterns;
  patterns.AddPattern(LabelPattern(LabelPattern::RECURSIVE_DIRECTORY,
                                   SourceDir("//foo/bar/"), "", Label()));

  EXPECT_TRUE(patterns.Matches(Label(SourceDir("//foo/bar/"), "target")));
  EXPECT_TRUE(patterns.Matches(Label(SourceDir("//foo/bar/sub/"), "target")));
  EXPECT_TRUE(
      patterns.Matches(Label(SourceDir("//foo/bar/sub/deep/"), "target")));

  EXPECT_FALSE(patterns.Matches(Label(SourceDir("//foo/"), "target")));
  EXPECT_FALSE(patterns.Matches(Label(SourceDir("//other/"), "target")));
}

TEST(LabelPatternsTest, UniversalWildcard) {
  LabelPatterns root_patterns;
  root_patterns.AddPattern(LabelPattern(LabelPattern::RECURSIVE_DIRECTORY,
                                        SourceDir("//"), "", Label()));
  EXPECT_TRUE(root_patterns.Matches(Label(SourceDir("//any/dir/"), "target")));
  EXPECT_TRUE(root_patterns.Matches(Label(SourceDir("//"), "root")));
}