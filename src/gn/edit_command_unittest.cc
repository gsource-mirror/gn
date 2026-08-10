// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <string>
#include <string_view>
#include <vector>

#include "base/files/file_util.h"
#include "base/files/scoped_temp_dir.h"
#include "gn/edit_command.h"
#include "gn/err.h"
#include "gn/filesystem_utils.h"
#include "gn/setup.h"
#include "gn/test_with_scheduler.h"
#include "util/test/test.h"

namespace commands {
namespace {

// Runs an edit command on matching the given target
// patterns, and returns the formatted output file contents.
Result<std::string> DoEdit(std::string command,
                           std::vector<std::string> patterns,
                           const std::string& before) {
  base::ScopedTempDir temp_dir;
  if (!temp_dir.CreateUniqueTempDir()) {
    return Err(Location(), "Failed to create temp dir");
  }
  base::FilePath root_path = base::MakeAbsoluteFilePath(temp_dir.GetPath());

  base::FilePath build_gn_path = root_path.AppendASCII("BUILD.gn");
  if (!WriteFile(build_gn_path, before, nullptr)) {
    return Err(Location(), "Failed to write BUILD.gn");
  }
  base::FilePath dot_gn_path = root_path.AppendASCII(".gn");
  if (!WriteFile(dot_gn_path, "", nullptr)) {
    return Err(Location(), "Failed to write .gn");
  }

  Setup setup;
  setup.build_settings().SetRootPath(root_path);

  std::vector<std::string> args;
  args.push_back(std::move(command));
  for (auto& p : patterns) {
    args.push_back(std::move(p));
  }

  auto result = RunEditImpl(args, setup);
  if (result.has_error()) {
    return result.error();
  }

  std::string after;
  if (!base::ReadFileToString(build_gn_path, &after)) {
    return Err(Location(), "Failed to read BUILD.gn");
  }
  return after;
}

// Runs an edit command on matching target pattern "//..." (everything).
Result<std::string> DoEdit(std::string command, const std::string& before) {
  return DoEdit(std::move(command), {"//:*"}, before);
}

// Doesn't actually do anything, but makes the code look more pretty by
// matching the indent and creating a clear division between input and
// expected output.
std::string Edited(std::string before) {
  return before;
}

}  // namespace

using EditCommandTest = TestWithScheduler;

TEST_F(EditCommandTest, MultipleTargetsSubset) {
  EXPECT_SUCCESS(DoEdit("set testonly true", {"//:foo"},
                        "executable(\"foo\") {\n"
                        "  testonly = false\n"
                        "}\n"
                        "executable(\"bar\") {\n"
                        "  testonly = false\n"
                        "}\n"),
                 Edited("executable(\"foo\") {\n"
                        "  testonly = true\n"
                        "}\n"
                        "executable(\"bar\") {\n"
                        "  testonly = false\n"
                        "}\n"));
}

TEST_F(EditCommandTest, PatternNeverMatched) {
  EXPECT_FAILURE(DoEdit("set testonly true", {"//:nonexistent"},
                        "executable(\"foo\") {\n"
                        "}\n"),
                 "Target(s) not found: //:nonexistent");
}

TEST_F(EditCommandTest, SetSubcommand) {
  // New bool attribute
  EXPECT_SUCCESS(DoEdit("set testonly true",
                        "executable(\"foo\") {\n"
                        "}\n"),
                 Edited("executable(\"foo\") {\n"
                        "  testonly = true\n"
                        "}\n"));

  // Replacing existing attribute
  EXPECT_SUCCESS(DoEdit("set testonly true",
                        "executable(\"foo\") {\n"
                        "  testonly = false\n"
                        "}\n"),
                 Edited("executable(\"foo\") {\n"
                        "  testonly = true\n"
                        "}\n"));

  // String attribute
  EXPECT_SUCCESS(DoEdit("set label \"//foo:bar\"",
                        "executable(\"foo\") {\n"
                        "}\n"),
                 Edited("executable(\"foo\") {\n"
                        "  label = \"//foo:bar\"\n"
                        "}\n"));

  // Int attribute
  EXPECT_SUCCESS(DoEdit("set assert_no_deps 42",
                        "executable(\"foo\") {\n"
                        "}\n"),
                 Edited("executable(\"foo\") {\n"
                        "  assert_no_deps = 42\n"
                        "}\n"));

  // Conditional assignment
  EXPECT_SUCCESS(
      DoEdit("set testonly true",
             "executable(\"foo\") {\n"
             "  if (is_linux) {\n"
             "    testonly = true\n"
             "  }\n"
             "}\n"),
      Edited("executable(\"foo\") {\n"
             "  if (is_linux) {\n"
             "    # TODO(gn edit): This is conditional, so double check "
             "this is safe to remove\n"
             "    testonly = true\n"
             "  }\n"
             "  testonly = true\n"
             "}\n"));

  // Modification assignment
  EXPECT_SUCCESS(
      DoEdit("set deps \"[ \\\"//baz\\\" ]\"",
             "executable(\"foo\") {\n"
             "  deps += [ \"//bar\" ]\n"
             "}\n"),
      Edited("executable(\"foo\") {\n"
             "  # TODO(gn edit): This is a modification, so double check "
             "this is safe to\n"
             "  # remove\n"
             "  deps += [ \"//bar\" ]\n"
             "  deps = [ \"//baz\" ]\n"
             "}\n"));
}

}  // namespace commands
