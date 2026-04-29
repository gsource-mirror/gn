// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <iostream>
#include <string>
#include <vector>

#include "base/containers/span.h"
#include "base/strings/string_split.h"
#include "base/strings/string_util.h"
#include "gn/commands.h"
#include "gn/setup.h"
#include "gn/standard_out.h"
#include "gn/target.h"

namespace commands {

const char kShell[] = "shell";
const char kShell_HelpShort[] = "shell: Run an interactive GN shell.";
const char kShell_Help[] =
    R"(shell: Run an interactive GN shell.

  gn shell <out_dir>

  Starts an interactive prompt that allows running GN commands efficiently,
  without having to reload the build graph for each command.

  Currently only supports a subset of GN functionality, like:
    suggest path/to/target.cc=foo/bar.h
)";

int RunShell(const std::vector<std::string>& args) {
  if (args.size() != 1) {
    OutputString(
        "gn shell requires exactly one argument (the output directory).\n",
        TextDecoration::DECORATION_RED);
    return 1;
  }

  // Perform the slow setup operations once.
  // Deliberately leaked to avoid expensive process teardown.
  Setup* setup = new Setup;
  if (!setup->DoSetup(args[0], false) || !setup->Run())
    return 1;

  std::vector<const Target*> all_targets =
      setup->builder().GetAllResolvedTargets();

  std::string line;
  while (true) {
    OutputString(">>> ");
    // Check for EOF
    if (!std::getline(std::cin, line)) {
      break;
    }

    std::vector<std::string> cmd_args =
        base::SplitString(line, base::kWhitespaceASCII, base::TRIM_WHITESPACE,
                          base::SPLIT_WANT_NONEMPTY);

    // Standard shell convention is to just repeat if the line was empty.
    if (cmd_args.empty())
      continue;

    const std::string& cmd = cmd_args[0];
    auto args = base::make_span(cmd_args).subspan(1);
    if (cmd == "exit" || cmd == "quit") {
      break;
    } else if (cmd == "suggest") {
      RunSuggestInner(setup, all_targets, args);
    } else if (cmd == "refs") {
      RunRefsInner(setup, all_targets, args);
    } else if (cmd == "ls") {
      RunLsInner(setup, all_targets, args);
    } else if (cmd == "desc") {
      RunDescInner(setup, all_targets, args);
    } else if (cmd == "meta") {
      RunMetaInner(setup, all_targets, args);
    } else if (cmd == "outputs") {
      RunOutputsInner(setup, all_targets, args);
    } else if (cmd == "path") {
      RunPathInner(setup, all_targets, args);
    } else if (cmd == "analyze") {
      RunAnalyzeInner(setup, all_targets, args);
    } else {
      OutputString("Unknown shell command: " + cmd + "\n",
                   TextDecoration::DECORATION_RED);
    }
  }

  return 0;
}

}  // namespace commands
