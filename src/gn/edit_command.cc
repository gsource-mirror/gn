// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/edit_command.h"
#include "gn/edit/edit_set.h"

#include <algorithm>
#include <cctype>
#include <iomanip>
#include <ranges>
#include <sstream>

#include "base/files/file_util.h"
#include "base/strings/string_number_conversions.h"
#include "base/strings/string_util.h"
#include "gn/command_format.h"
#include "gn/commands.h"
#include "gn/edit/build_file_resolver.h"
#include "gn/filesystem_utils.h"
#include "gn/input_file.h"
#include "gn/parse_tree.h"
#include "gn/parser.h"
#include "gn/setup.h"
#include "gn/source_file.h"
#include "gn/tokenizer.h"
#include "gn/value.h"

namespace commands {

namespace {

Result<std::unique_ptr<EditCommand>> ParseCommand(
    std::vector<std::string> args) {
  if (args.empty()) {
    return Err(Location(), "Empty command.");
  }

  if (args[0] == "set") {
    if (args.size() != 3) {
      return Err(Location(), "Invalid set command.",
                 "Usage: set <attribute> <value>");
    }
    return std::unique_ptr<EditCommand>(
        std::make_unique<SetCommand>(args[1], args[2]));
  }

  return Err(Location(), "Unknown command action: " + args[0],
             "Currently only \"set\" is supported.");
}

}  // namespace

const char kEdit[] = "edit";
const char kEdit_HelpShort[] =
    "edit: Edit BUILD.gn files from the command line.";
const char kEdit_Help[] =
    "gn edit <command> <labels...>\n"
    "\n"
    "  Edits GN files by applying a single action to targets matched by "
    "patterns.\n"
    "\n"
    "Commands:\n"
    "  set <attribute> <value>\n"
    "      Sets/overwrites target's <attribute> to <value>.\n"
    "      E.g. set deps //foo:bar\n"
    "\n"
    "Examples:\n"
    "  gn edit \"set deps //src/base\" //src/tools:*\n"
    "      Sets 'deps' to ['//src/base'] for all targets in "
    "//src/tools/BUILD.gn.\n"
    "\n"
    "  gn edit \"set configs //build:custom\" :my_target\n"
    "      Sets 'configs' to ['//build:custom'] for :my_target in the current "
    "directory.\n";

Err EditCommandImpl(const std::vector<std::string>& args, Setup& setup) {
  if (args.size() < 2) {
    return Err(Location(), "Insufficient arguments.",
               "Usage: gn edit <command> <labels...>\n"
               "Example: gn edit \"set testonly true\" //foo:*");
  }

  // We use std::quoted to tokenize the command.
  // eg. set foo "bar baz" -> ["set", "foo", "bar baz"].
  std::stringstream ss(args[0]);
  std::vector<std::string> command_tokens;
  std::string token;
  while (ss >> std::ws && !ss.eof()) {
    if (!(ss >> std::quoted(token))) {
      return Err(Location(), "Unclosed quote in command string.");
    }
    command_tokens.push_back(token);
  }
  if (command_tokens.empty()) {
    return Err(Location(), "Empty command string.");
  }

  ASSIGN_OR_RETURN(std::unique_ptr<EditCommand> command,
                   ParseCommand(std::move(command_tokens)));
  const SourceDir current_dir =
      SourceDirForCurrentDirectory(setup.build_settings().root_path());
  const std::string source_root = setup.build_settings().root_path_utf8();

  std::vector<LabelPattern> patterns;
  for (size_t i = 1; i < args.size(); ++i) {
    Value val(nullptr, args[i]);
    Err err;
    LabelPattern pattern =
        LabelPattern::GetPattern(current_dir, source_root, val, &err);
    if (err.has_error()) {
      return err;
    }
    patterns.push_back(pattern);
  }

  ASSIGN_OR_RETURN(std::vector<BuildFile> build_files,
                   ResolvePatternsToBuildFiles(&setup.build_settings(),
                                               setup.loader(), patterns));

  for (auto& build_file : build_files) {
    RETURN_IF_ERROR(command->Apply(build_file));
    RETURN_IF_ERROR(build_file.label_matcher().done());
  }

  // 5. Format and write files back to disk.
  for (const auto& build_file : build_files) {
    ASSIGN_OR_RETURN(std::string formatted,
                     FormatNodeToString(build_file.root()));

    base::FilePath file_path =
        setup.build_settings().GetFullPath(build_file.source_file());
    if (base::WriteFile(file_path, formatted.data(),
                        static_cast<int>(formatted.size())) == -1) {
      return Err(Location(),
                 "Failed to write to file: " + FilePathToUTF8(file_path));
    }
    printf("Wrote '%s'.\n", FilePathToUTF8(file_path).c_str());
  }

  return Err();
}

int RunEdit(const std::vector<std::string>& args) {
  Setup setup;
  if (!setup.DoSetupForFormat()) {
    return 1;
  }
  Err err = EditCommandImpl(args, setup);
  if (err.has_error()) {
    if (!err.message().empty()) {
      err.PrintToStdout();
    }
    return 1;
  }
  return 0;
}

}  // namespace commands
