// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_EDIT_EDIT_TEST_HELPER_H_
#define TOOLS_GN_EDIT_EDIT_TEST_HELPER_H_

#include <string>
#include <string_view>
#include <vector>

#include "base/files/file_util.h"
#include "base/files/scoped_temp_dir.h"
#include "gn/edit_command.h"
#include "gn/err.h"
#include "gn/filesystem_utils.h"
#include "gn/scheduler.h"
#include "gn/setup.h"
#include "util/msg_loop.h"

namespace commands {

// Runs an edit command on SourceFile="//BUILD.gn" matching the given target
// patterns, and returns the formatted output file contents.
inline Result<std::string> RunEditCommand(
    std::string command,
    const std::vector<std::string>& patterns,
    std::string before) {
  MsgLoop run_loop;
  Scheduler scheduler;

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
  for (const auto& p : patterns) {
    args.push_back(p);
  }

  Err err = EditCommandImpl(args, setup);
  if (err.has_error()) {
    return err;
  }

  std::string after;
  if (!base::ReadFileToString(build_gn_path, &after)) {
    return Err(Location(), "Failed to read BUILD.gn");
  }
  return after;
}

// Runs an edit command on SourceFile="//BUILD.gn" matching target pattern
// "//..." (everything).
inline Result<std::string> RunEditCommand(std::string command,
                                          std::string before) {
  return RunEditCommand(std::move(command), {"//:*"}, std::move(before));
}

}  // namespace commands

#endif  // TOOLS_GN_EDIT_EDIT_TEST_HELPER_H_
