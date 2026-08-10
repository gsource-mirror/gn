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
struct Edited;
std::string Pretty(const Edited& edited);

struct Edited {
  Edited(std::string_view contents, EditState edit_state = EditState())
      : contents_(contents), edit_state_(std::move(edit_state)) {}

  bool operator==(const Edited& other) const {
    return Pretty(*this) == Pretty(other);
  }

  std::string contents_;
  EditState edit_state_;
};

std::string Pretty(const Edited& edited) {
  std::string res = edited.contents_;
  if (!edited.edit_state_.needs_manual_review.empty()) {
    res += "\nNeeds manual review: " +
           testing::Pretty(edited.edit_state_.needs_manual_review);
  }
  if (!edited.edit_state_.warnings.empty()) {
    res += "\nWarnings: " + testing::Pretty(edited.edit_state_.warnings);
  }
  return res;
}

// Runs an edit command on matching the given target
// patterns, and returns the formatted output file contents.
Result<Edited> DoEdit(std::string command,
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
  return Edited(after, std::move(result->second));
}

// Runs an edit command matching all targets in the root BUILD.gn ("//:*").
Result<Edited> DoEdit(std::string command, const std::string& before) {
  return DoEdit(std::move(command), {"//:*"}, before);
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
  EXPECT_SUCCESS(DoEdit("set testonly false",
                        "executable(\"foo\") {\n"
                        "  testonly = true\n"
                        "}\n"),
                 Edited("executable(\"foo\") {\n"
                        "  testonly = false\n"
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

  // Multiple values setting a list (replaces first, deletes modification,
  // adds review for conditional)
  EXPECT_SUCCESS(
      DoEdit("set deps //foo //bar",
             "executable(\"foo\") {\n"
             "  deps = [ \"//bar\" ]\n"
             "  deps += [ \"//baz\" ]\n"
             "  if (is_linux) {\n"
             "    deps += [ \"//linux\" ]\n"
             "  }\n"
             "}\n"),
      Edited(
          "executable(\"foo\") {\n"
          "  deps = [\n"
          "    \"//bar\",\n"
          "    \"//foo\",\n"
          "  ]\n"
          "\n"
          "  if (is_linux) {\n"
          "    # TODO(gn edit: set deps //foo //bar):\n"
          "    # This would normally be deleted but is conditional.\n"
          "    # Manual intervention is required to decide whether it should "
          "actually be deleted.\n"
          "    deps += [ \"//linux\" ]\n"
          "  }\n"
          "}\n",
          EditState({Label(SourceDir("//"), "foo")})));

  // Forced list attribute (appends new list, adds review for conditional)
  EXPECT_SUCCESS(
      DoEdit("set deps:list //foo",
             "executable(\"foo\") {\n"
             "  if (is_linux) {\n"
             "    deps = [ \"//linux\" ]\n"
             "    public_deps = [ \"//linux\" ]\n"
             "  }\n"
             "}\n"),
      Edited(
          "executable(\"foo\") {\n"
          "  if (is_linux) {\n"
          "    # TODO(gn edit: set deps:list //foo):\n"
          "    # This would normally be deleted but is conditional.\n"
          "    # Manual intervention is required to decide whether it should "
          "actually be deleted.\n"
          "    deps = [ \"//linux\" ]\n"
          "    public_deps = [ \"//linux\" ]\n"
          "  }\n"
          "  deps = [ \"//foo\" ]\n"
          "}\n",
          EditState({Label(SourceDir("//"), "foo")})));
}

}  // namespace commands
