// Copyright (c) 2013 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ninja_toolchain_writer.h"

#include <fstream>

#include "base/files/file_util.h"
#include "base/strings/stringize_macros.h"
#include "gn/build_settings.h"
#include "gn/builtin_tool.h"
#include "gn/c_tool.h"
#include "gn/filesystem_utils.h"
#include "gn/general_tool.h"
#include "gn/ninja_utils.h"
#include "gn/output_file.h"
#include "gn/pool.h"
#include "gn/settings.h"
#include "gn/source_file.h"
#include "gn/string_output_buffer.h"
#include "gn/substitution_writer.h"
#include "gn/target.h"
#include "gn/toolchain.h"
#include "gn/trace.h"

namespace {

const char kIndent[] = "  ";

}  // namespace

NinjaToolchainWriter::NinjaToolchainWriter(const Settings* settings,
                                           const Toolchain* toolchain,
                                           std::ostream& out)
    : settings_(settings),
      toolchain_(toolchain),
      out_(out),
      path_output_(settings_->build_settings()->build_dir(),
                   settings_->build_settings()->root_path_utf8(),
                   ESCAPE_NINJA) {}

NinjaToolchainWriter::~NinjaToolchainWriter() = default;

void NinjaToolchainWriter::RunToolRules() {
  std::string rule_prefix = GetNinjaRulePrefixForToolchain(settings_);

  for (const auto& tool : toolchain_->tools()) {
    if (tool.second->name() == GeneralTool::kGeneralToolAction ||
        tool.second->AsBuiltin()) {
      continue;
    }
    WriteToolRule(tool.second.get(), rule_prefix);
  }
}

// static
bool NinjaToolchainWriter::RunAndWriteFile(
    const Settings* settings,
    const Toolchain* toolchain,
    const std::vector<NinjaWriter::TargetRulePair>& rules) {
  // Group rules by SourceDir.
  // The rules are already sorted by target->label(), meaning targets in each
  // directory are contiguous and sorted alphabetically by target name.
  std::vector<std::pair<SourceDir, std::string>> dir_rules;
  for (const auto& pair : rules) {
    const SourceDir& dir = pair.first->label().dir();
    if (dir_rules.empty() || dir_rules.back().first != dir) {
      dir_rules.emplace_back(dir, std::string());
    }
    dir_rules.back().second.append(pair.second);
  }

  std::vector<SourceFile> written_subninjas;
  for (const auto& dir_entry : dir_rules) {
    if (dir_entry.second.empty())
      continue;

    SourceFile ninja_file = GetNinjaFileForBuildFile(settings, dir_entry.first);
    base::FilePath full_ninja_file =
        settings->build_settings()->GetFullPath(ninja_file);

    StringOutputBuffer storage;
    storage.Append(dir_entry.second);
    storage.WriteToFileIfChanged(full_ninja_file, nullptr);

    written_subninjas.push_back(ninja_file);
  }

  base::FilePath ninja_file(settings->build_settings()->GetFullPath(
      GetNinjaFileForToolchain(settings)));
  ScopedTrace trace(TraceItem::TRACE_FILE_WRITE_NINJA,
                    FilePathToUTF8(ninja_file));

  base::CreateDirectory(ninja_file.DirName());

  StringOutputBuffer toolchain_storage;
  std::ostream file(&toolchain_storage);

  NinjaToolchainWriter gen(settings, toolchain, file);
  gen.RunToolRules();
  file << std::endl;

  EscapeOptions options;
  options.mode = ESCAPE_NINJA;
  for (const auto& subninja_file : written_subninjas) {
    file << "subninja ";
    file << EscapeString(
        OutputFile(settings->build_settings(), subninja_file).value(), options,
        nullptr);
    file << "\n";
  }

  toolchain_storage.WriteToFileIfChanged(ninja_file, nullptr);
  return true;
}

void NinjaToolchainWriter::WriteToolRule(Tool* tool,
                                         const std::string& rule_prefix) {
  out_ << "rule " << rule_prefix << tool->name() << std::endl;

  // Rules explicitly include shell commands, so don't try to escape.
  EscapeOptions options;
  options.mode = ESCAPE_NINJA_PREFORMATTED_COMMAND;

  WriteCommandRulePattern("command", tool->command_launcher(), tool->command(),
                          options);

  WriteRulePattern("description", tool->description(), options);
  WriteRulePattern("rspfile", tool->rspfile(), options);
  WriteRulePattern("rspfile_content", tool->rspfile_content(), options);

  if (CTool* c_tool = tool->AsC()) {
    if (c_tool->depsformat() == CTool::DEPS_GCC) {
      // GCC-style deps require a depfile.
      if (!c_tool->depfile().empty()) {
        WriteRulePattern("depfile", tool->depfile(), options);
        out_ << kIndent << "deps = gcc" << std::endl;
      }
    } else if (c_tool->depsformat() == CTool::DEPS_MSVC) {
      // MSVC deps don't have a depfile.
      out_ << kIndent << "deps = msvc" << std::endl;
    }
  } else if (!tool->depfile().empty()) {
    WriteRulePattern("depfile", tool->depfile(), options);
    out_ << kIndent << "deps = gcc" << std::endl;
  }

  // Use pool is specified.
  if (tool->pool().ptr) {
    std::string pool_name =
        tool->pool().ptr->GetNinjaName(settings_->default_toolchain_label());
    out_ << kIndent << "pool = " << pool_name << std::endl;
  }

  if (tool->restat())
    out_ << kIndent << "restat = 1" << std::endl;

  // If the size is exactly 1, we don't need a phony rule, since we just write
  // the input file directly in the build action.
  if (tool->inputs().size() > 1) {
    out_ << "build ";
    path_output_.WriteFile(
        out_,
        *tool->inputs_phony_or_file(rule_prefix, *settings_->build_settings()));
    out_ << ": phony";
    for (const auto& input : tool->inputs()) {
      out_ << " ";
      path_output_.WriteFile(out_,
                             OutputFile(settings_->build_settings(), input));
    }
    out_ << std::endl;
  }
}

void NinjaToolchainWriter::WriteRulePattern(const char* name,
                                            const SubstitutionPattern& pattern,
                                            const EscapeOptions& options) {
  if (pattern.empty())
    return;
  out_ << kIndent << name << " = ";
  SubstitutionWriter::WriteWithNinjaVariables(pattern, options, out_);
  out_ << std::endl;
}

void NinjaToolchainWriter::WriteCommandRulePattern(
    const char* name,
    const std::string& launcher,
    const SubstitutionPattern& command,
    const EscapeOptions& options) {
  CHECK(!command.empty()) << "Command should not be empty";
  out_ << kIndent << name << " = ";
  if (!launcher.empty())
    out_ << launcher << " ";
  SubstitutionWriter::WriteWithNinjaVariables(command, options, out_);
  out_ << std::endl;
}
