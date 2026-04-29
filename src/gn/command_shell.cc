// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <algorithm>
#include <iostream>
#include <string>
#include <vector>

#include "base/command_line.h"
#include "base/strings/string_split.h"
#include "base/strings/string_util.h"
#include "gn/commands.h"
#include "gn/setup.h"
#include "gn/standard_out.h"
#include "util/msg_loop.h"

namespace commands {

const char kGn[] = "gn";
const char kShell[] = "shell";
const char kShell_HelpShort[] = "shell: Run an interactive GN shell.";
const char kShell_Help[] =
    R"(shell: Run an interactive GN shell.

  gn shell <out_dir>

  Starts an interactive prompt that allows running GN commands efficiently,
  without having to reload the build graph for each command.

  The following commands are supported:
  * analyze
  * desc
  * ls
  * meta
  * outputs
  * path
  * refs
  * suggest

  Type 'quit', 'exit', or Control-D (EOF) to leave the shell.
)";

std::array<std::string_view, 8> kShellCommands = {
    kAnalyze, kDesc, kLs, kMeta, kOutputs, kPath, kRefs, kSuggest,
};

class CachedSetup : public Setup {
 public:
  bool DoSetup(const std::string& build_dir, bool force_create) override {
    return true;
  }

  bool Run() override { return true; }
};

int RunShell(Setup*, const std::vector<std::string>& args) {
  if (args.size() != 1) {
    Err(Location(), "gn shell requires exactly one argument.",
        "Usage: \"gn shell <out_dir>\"")
        .PrintToStdout();
    return 1;
  }

  // Perform the slow setup operations once.
  // Deliberately leaked to avoid expensive process teardown.
  CachedSetup* setup = new CachedSetup;
  if (!setup->Setup::DoSetup(args[0], false) || !setup->Setup::Run())
    return 1;

  MsgLoop* loop = MsgLoop::Current();
  // One message loop exists per command. We can't run an inner command without
  // first cleaning up `gn shell`'s message loop.
  // This should be fine since we are finished with the message loop now.
  loop->~MsgLoop();

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

    // Standard shell convention is to just re-prompt if the line was empty.
    if (cmd_args.empty()) {
      continue;
    }

    const auto& cmd = cmd_args[0];
    if (cmd == "quit" || cmd == "exit") {
      break;
    }
    if (std::ranges::find(kShellCommands, cmd) == kShellCommands.end()) {
      Err(Location(), "Unknown shell command: " + cmd,
          "Use 'gn help shell' to see available commands.")
          .PrintToStdout();
      continue;
    }
    std::vector<const char*> argv = {kGn, cmd.c_str(), args[0].c_str()};
    for (auto i = 1U; i < cmd_args.size(); i++) {
      argv.push_back(cmd_args[i].c_str());
    }

    base::CommandLine::Reset();
    CommandSwitches::Set(CommandSwitches());
    gn_main(argv.size(), const_cast<char**>(argv.data()), setup);
  }

  // Restore GN shell's message loop
  new (loop) MsgLoop();
  return 0;
}

}  // namespace commands
