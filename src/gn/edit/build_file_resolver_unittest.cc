// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/edit/build_file_resolver.h"

#include <algorithm>

#include "base/files/file_util.h"
#include "base/files/scoped_temp_dir.h"
#include "gn/build_settings.h"
#include "gn/filesystem_utils.h"
#include "gn/label_pattern.h"
#include "gn/loader.h"
#include "gn/source_file.h"
#include "gn/test_with_scope.h"
#include "util/test/test.h"

namespace commands {

class BuildFileResolverTest : public testing::Test {
 protected:
  BuildFileResolverTest() {
    CHECK(temp_dir_.CreateUniqueTempDir());
    build_settings_.SetRootPath(temp_dir_.GetPath());
    loader_ = scoped_refptr<LoaderImpl>(new LoaderImpl(&build_settings_));
  }

  base::ScopedTempDir temp_dir_;
  BuildSettings build_settings_;
  scoped_refptr<LoaderImpl> loader_;
};

TEST_F(BuildFileResolverTest, ResolveExactAndDirectoryPatterns) {
  SourceDir current_dir("//");
  std::string source_root = FilePathToUTF8(temp_dir_.GetPath());

  // Create mock BUILD.gn files.
  base::FilePath foo_dir = temp_dir_.GetPath().AppendASCII("foo");
  ASSERT_TRUE(base::CreateDirectory(foo_dir));
  ASSERT_TRUE(WriteFile(foo_dir.AppendASCII("BUILD.gn"),
                        "shared_library(\"bar\") { }\n", nullptr));

  // Resolve patterns:
  // 1. MATCH //foo:bar
  // 2. DIRECTORY //foo:*
  Err err;
  LabelPattern match_pattern = LabelPattern::GetPattern(
      current_dir, source_root, Value(nullptr, "//foo:bar"), &err);
  ASSERT_SUCCESS(err);

  LabelPattern dir_pattern = LabelPattern::GetPattern(
      current_dir, source_root, Value(nullptr, "//foo:*"), &err);
  ASSERT_SUCCESS(err);

  auto result = ResolvePatternsToBuildFiles(&build_settings_, loader_.get(),
                                            {match_pattern, dir_pattern});
  ASSERT_SUCCESS(result);

  // Both should resolve to the same unique BuildFile instance because they are
  // in the same dir.
  ASSERT_EQ(result->size(), 1u);

  auto& build_file = (*result)[0];
  EXPECT_EQ(build_file.source_file().value(), "//foo/BUILD.gn");
  EXPECT_TRUE(build_file.root());
}

TEST_F(BuildFileResolverTest, ResolveRecursivePatterns) {
  SourceDir current_dir("//");
  std::string source_root = FilePathToUTF8(temp_dir_.GetPath());

  // Create nested BUILD.gn files:
  // //foo/BUILD.gn
  // //foo/bar/BUILD.gn
  // //baz/BUILD.gn (should not match //foo/...)
  base::FilePath foo_dir = temp_dir_.GetPath().AppendASCII("foo");
  base::FilePath bar_dir = foo_dir.AppendASCII("bar");
  base::FilePath baz_dir = temp_dir_.GetPath().AppendASCII("baz");
  ASSERT_TRUE(base::CreateDirectory(bar_dir));
  ASSERT_TRUE(base::CreateDirectory(baz_dir));

  ASSERT_TRUE(
      WriteFile(foo_dir.AppendASCII("BUILD.gn"), "group(\"g1\") {}", nullptr));
  ASSERT_TRUE(
      WriteFile(bar_dir.AppendASCII("BUILD.gn"), "group(\"g2\") {}", nullptr));
  ASSERT_TRUE(
      WriteFile(baz_dir.AppendASCII("BUILD.gn"), "group(\"g3\") {}", nullptr));

  Err err;
  LabelPattern rec_pattern = LabelPattern::GetPattern(
      current_dir, source_root, Value(nullptr, "//foo/*"), &err);
  ASSERT_SUCCESS(err);

  auto result = ResolvePatternsToBuildFiles(&build_settings_, loader_.get(),
                                            {rec_pattern});
  ASSERT_SUCCESS(result);

  ASSERT_EQ(result->size(), 2u);

  // Order of traversed files can be anything but they should be //foo/BUILD.gn
  // and //foo/bar/BUILD.gn.
  std::vector<std::string> paths = {(*result)[0].source_file().value(),
                                    (*result)[1].source_file().value()};
  std::sort(paths.begin(), paths.end());
  EXPECT_EQ(paths[0], "//foo/BUILD.gn");
  EXPECT_EQ(paths[1], "//foo/bar/BUILD.gn");
}

TEST_F(BuildFileResolverTest, CustomBuildFileExtension) {
  SourceDir current_dir("//");
  std::string source_root = FilePathToUTF8(temp_dir_.GetPath());

  // Configure custom build file extension.
  loader_->set_build_file_extension("myproj");

  // Create custom BUILD.myproj.gn file.
  base::FilePath foo_dir = temp_dir_.GetPath().AppendASCII("foo");
  ASSERT_TRUE(base::CreateDirectory(foo_dir));
  ASSERT_TRUE(WriteFile(foo_dir.AppendASCII("BUILD.myproj.gn"),
                        "group(\"bar\") {}\n", nullptr));

  Err err;
  LabelPattern match_pattern = LabelPattern::GetPattern(
      current_dir, source_root, Value(nullptr, "//foo:bar"), &err);
  ASSERT_SUCCESS(err);

  auto result = ResolvePatternsToBuildFiles(&build_settings_, loader_.get(),
                                            {match_pattern});
  ASSERT_SUCCESS(result);

  ASSERT_EQ(result->size(), 1u);
  EXPECT_EQ((*result)[0].source_file().value(), "//foo/BUILD.myproj.gn");
}

}  // namespace commands
