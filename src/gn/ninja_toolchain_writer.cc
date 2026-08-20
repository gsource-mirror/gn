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
#include "gn/c_substitution_type.h"
#include "gn/ninja_target_writer.h"
#include "gn/ninja_utils.h"
#include "gn/output_file.h"
#include "gn/pool.h"
#include "gn/rust_substitution_type.h"
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
  // Group targets by SourceDir.
  // The rules are already sorted by target->label(), meaning targets in each
  // directory are contiguous and sorted alphabetically by target name.
  std::vector<std::pair<SourceDir, std::vector<const Target*>>> dir_targets;
  for (const auto& pair : rules) {
    const SourceDir& dir = pair.first->label().dir();
    if (dir_targets.empty() || dir_targets.back().first != dir) {
      dir_targets.emplace_back(dir, std::vector<const Target*>());
    }
    dir_targets.back().second.push_back(pair.first);
  }

  PathOutput path_output(SourceDir(),
                         settings->build_settings()->root_path_utf8(),
                         ESCAPE_NINJA_COMMAND);

  std::vector<SourceFile> written_subninjas;
  for (const auto& dir_entry : dir_targets) {
    const auto& targets = dir_entry.second;
    if (targets.empty())
      continue;

    // Collect binary targets to compute common variables.
    std::vector<const Target*> binary_targets;
    for (const Target* t : targets) {
      if (t->IsBinary())
        binary_targets.push_back(t);
    }

    CommonVars common_vars;
    if (!binary_targets.empty()) {
      // Defines.
      std::string first_defines =
          NinjaTargetWriter::ComputeDefines(binary_targets[0]);
      if (!first_defines.empty()) {
        bool all_match = true;
        for (size_t i = 1; i < binary_targets.size(); ++i) {
          if (NinjaTargetWriter::ComputeDefines(binary_targets[i]) !=
              first_defines) {
            all_match = false;
            break;
          }
        }
        if (all_match)
          common_vars.vars[&CSubstitutionDefines] = first_defines;
      }

      // Include directories.
      std::string first_includes =
          NinjaTargetWriter::ComputeIncludeDirs(binary_targets[0], path_output);
      if (!first_includes.empty()) {
        bool all_match = true;
        for (size_t i = 1; i < binary_targets.size(); ++i) {
          if (NinjaTargetWriter::ComputeIncludeDirs(binary_targets[i],
                                                   path_output) !=
              first_includes) {
            all_match = false;
            break;
          }
        }
        if (all_match)
          common_vars.vars[&CSubstitutionIncludeDirs] = first_includes;
      }

      // C/C++ / ASM flags.
      const Substitution* c_substs[] = {
          &CSubstitutionCFlags,      &CSubstitutionCFlagsC,
          &CSubstitutionCFlagsCc,    &CSubstitutionCFlagsObjC,
          &CSubstitutionCFlagsObjCc, &CSubstitutionAsmFlags,
      };
      for (const Substitution* s : c_substs) {
        std::string first_val =
            NinjaTargetWriter::ComputeCFlags(binary_targets[0], s, path_output);
        if (!first_val.empty()) {
          bool all_match = true;
          for (size_t i = 1; i < binary_targets.size(); ++i) {
            if (NinjaTargetWriter::ComputeCFlags(binary_targets[i], s,
                                                path_output) != first_val) {
              all_match = false;
              break;
            }
          }
          if (all_match)
            common_vars.vars[s] = first_val;
        }
      }

      // Rust flags.
      std::string first_rustflags =
          NinjaTargetWriter::ComputeRustFlags(binary_targets[0], path_output);
      if (!first_rustflags.empty()) {
        bool all_match = true;
        for (size_t i = 1; i < binary_targets.size(); ++i) {
          if (NinjaTargetWriter::ComputeRustFlags(binary_targets[i],
                                                 path_output) !=
              first_rustflags) {
            all_match = false;
            break;
          }
        }
        if (all_match)
          common_vars.vars[&kRustSubstitutionRustFlags] = first_rustflags;
      }
    }

    StringOutputBuffer storage;
    for (const auto& [subst, val] : common_vars.vars) {
      storage.Append(subst->ninja_name);
      storage.Append(" =");
      storage.Append(val);
      storage.Append("\n");
    }
    if (!common_vars.vars.empty())
      storage.Append("\n");

    for (const Target* target : targets) {
      std::string rule = NinjaTargetWriter::RunAndWriteFile(
          target, nullptr, nullptr, &common_vars);
      storage.Append(rule);
    }

    SourceFile ninja_file = GetNinjaFileForBuildFile(settings, dir_entry.first);
    base::FilePath full_ninja_file =
        settings->build_settings()->GetFullPath(ninja_file);
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
