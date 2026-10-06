// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "base/command_line.h"
#include "base/files/file_util.h"
#include "base/json/json_reader.h"
#include "base/json/json_writer.h"
#include "gn/commands.h"
#include "gn/compile_flags_writer.h"
#include "gn/err.h"
#include "gn/filesystem_utils.h"
#include "gn/standard_out.h"
#include "gn/switches.h"
#include "gn/trace.h"

namespace commands {

const char kMaterializeCompileFlags[] = "materialize-compile-flags";
const char kMaterializeCompileFlags_HelpShort[] =
    "materialize-compile-flags: Restore an exported snapshot without "
    "evaluating GN.";
const char kMaterializeCompileFlags_Help[] =
    R"(gn materialize-compile-flags <response-file> <output-file>

  Internal build helper. Reads resolved JSON supplied by a generated Ninja
  response file and atomically publishes a compile flag snapshot.
  Does not load .gn, evaluate build files, or execute configuration scripts.
)";

int RunMaterializeCompileFlags(const std::vector<std::string>& args) {
  const auto* command_line = base::CommandLine::ForCurrentProcess();
  if (command_line->HasSwitch(switches::kTime) ||
      command_line->HasSwitch(switches::kTracelog)) {
    EnableTracing();
  }
  if (args.size() != 2) {
    Err(Location(), "Expected a response file and an output file.")
        .PrintToStdout();
    return 1;
  }
  std::string contents;
  if (!base::ReadFileToString(UTF8ToFilePath(args[0]), &contents)) {
    Err(Location(), "Could not read compile flag response file.")
        .PrintToStdout();
    return 1;
  }
  auto value = base::JSONReader::Read(contents);
  if (!value || !value->is_dict()) {
    Err(Location(), "Invalid compile flag response JSON.").PrintToStdout();
    return 1;
  }
  std::string json;
  Err err;
  if (!base::JSONWriter::WriteWithOptions(
          *value, base::JSONWriter::OPTIONS_PRETTY_PRINT, &json) ||
      !WriteCompileFlagsAtomically(UTF8ToFilePath(args[1]), json, &err)) {
    if (!err.has_error()) {
      err = Err(Location(), "Could not serialize compile flags.");
    }
    err.PrintToStdout();
    return 1;
  }
  if (command_line->HasSwitch(switches::kTime)) {
    PrintLongHelp(SummarizeTraces());
  }
  if (command_line->HasSwitch(switches::kTracelog)) {
    SaveTraces(command_line->GetSwitchValuePath(switches::kTracelog));
  }
  return 0;
}

}  // namespace commands
