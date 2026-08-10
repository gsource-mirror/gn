// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/edit_command.h"
#include "gn/edit_subcommands.h"

#include <iomanip>
#include <sstream>

#include "base/files/file_util.h"
#include "gn/command_format.h"
#include "gn/commands.h"
#include "gn/edit/build_file_resolver.h"
#include "gn/filesystem_utils.h"
#include "gn/setup.h"
#include "gn/source_file.h"
#include "gn/value.h"

namespace commands {

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
    "      E.g. set testonly true\n"
    "\n"
    "Examples:\n"
    "  gn edit \"set testonly true\" //src/tools:*\n"
    "      Sets 'testonly' to 'true' for all targets in "
    "//src/tools/BUILD.gn.\n";

Result<std::vector<SourceFile>> RunEditImpl(
    const std::vector<std::string>& args,
    Setup& setup) {
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

  std::vector<SourceFile> modified_files;
  for (const auto& build_file : build_files) {
    ASSIGN_OR_RETURN(std::string formatted,
                     FormatNodeToString(build_file.root()));

    base::FilePath file_path =
        setup.build_settings().GetFullPath(build_file.source_file());

    std::string original_contents;
    base::ReadFileToString(file_path, &original_contents);

    if (original_contents != formatted) {
      if (base::WriteFile(file_path, formatted.data(),
                          static_cast<int>(formatted.size())) == -1) {
        return Err(Location(),
                   "Failed to write to file: " + FilePathToUTF8(file_path));
      }
      modified_files.push_back(build_file.source_file());
    }
  }

  return modified_files;
}

int RunEdit(const std::vector<std::string>& args) {
  Setup setup;
  if (!setup.DoSetupForFormat()) {
    return 1;
  }
  auto result = RunEditImpl(args, setup);
  if (result.has_error()) {
    if (!result.error().message().empty()) {
      result.error().PrintToStdout();
    }
    return 1;
  }
  for (const auto& file : *result) {
    printf("Wrote '%s'.\n", file.value().c_str());
  }
  return 0;
}

}  // namespace commands
