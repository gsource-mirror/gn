// Copyright (c) 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <stddef.h>
#include <vector>

#include "base/files/file_util.h"
#include "base/strings/string_split.h"
#include "base/strings/stringprintf.h"
#include "gn/commands.h"
#include "gn/filesystem_utils.h"
#include "gn/item.h"
#include "gn/setup.h"
#include "gn/standard_out.h"
#include "gn/target.h"

namespace commands {

const char kSuggest[] = "suggest";
const char kSuggest_HelpShort[] = "suggest: Suggest fixes to build graph based on includes.";
const char kSuggest_Help[] =
    R"(gn suggest

  gn suggest <out_dir> includer1=included1 includer2=included2...

  Where each includer or included is either a label, module name, or file path.

  Eg. gn suggest out_dir path/to/target.cc=foo/bar.h

  Will print a suggestion like:
  Request: path/to/target.cc wants to depend on foo/bar.h
  Suggestion: add deps = [ "//foo:bar" ] to "//path/to:target" (defined in //path/to/BUILD.gn:1234)
)";

constexpr std::string_view kPrivateSuffix = "_Private";

namespace {
// Determines whether a source file is in either the public or private API of a
// target.
std::optional<bool> GetDepTypeForSource(const Target* target,
                                        const SourceFile& file) {
  for (const auto& source : target->sources()) {
    if (source == file) {
      return target->all_headers_public() &&
                     file.GetType() == SourceFile::SOURCE_H
                 ? false
                 : true;
    }
  }
  for (const auto& header : target->public_headers()) {
    if (header == file) {
      return false;
    }
  }
  return std::nullopt;
}

// Finds all targets that use a file as a source from a specific toolchain and
// adds them to results. Checks every toolchain if current_toolchain is null.
bool AddToolchainSources(const std::vector<const Target*>& all_targets,
                         const Label* current_toolchain,
                         const SourceFile& file,
                         std::vector<std::pair<const Target*, bool>>& results) {
  for (const Target* target : all_targets) {
    if (!current_toolchain ||
        target->label().GetToolchainLabel() == *current_toolchain) {
      if (auto is_private = GetDepTypeForSource(target, file);
          is_private.has_value()) {
        results.emplace_back(target, *is_private);
      }
    }
  }
  return !results.empty();
}

SourceFile ResolveFilePath(const BuildSettings* build_settings,
                           std::string_view input_str) {
  if (input_str.starts_with("//")) {
    SourceFile file = SourceFile(input_str);
    if (base::PathExists(build_settings->GetFullPath(file))) {
      return file;
    }
    return SourceFile();
  }
  // Try relative to output directory first.
  // This is because the user is most likely running this based on an error
  // message from clang, which gives paths relative to the output directory to
  // be unambiguous.
  {
    Err err;
    SourceFile file = build_settings->build_dir().ResolveRelativeFile(
        Value(nullptr, std::string(input_str)), &err);
    if (!err.has_error() &&
        base::PathExists(build_settings->GetFullPath(file))) {
      return file;
    }
  }
  // Try relative to source root.
  {
    Err err;
    SourceFile file = SourceDir("//").ResolveRelativeFile(
        Value(nullptr, std::string(input_str)), &err);
    if (!err.has_error() &&
        base::PathExists(build_settings->GetFullPath(file))) {
      return file;
    }
  }
  return SourceFile();
}

constexpr auto kLabelLike = TextDecoration::DECORATION_GREEN;

void OutputSuggestion(const std::string& message) {
  OutputString("Suggestion: ", TextDecoration::DECORATION_BLUE);
  OutputString(message);
}

void OutputWarning(const std::string& message = "") {
  OutputString("Warning: ", TextDecoration::DECORATION_YELLOW);
  OutputString(message);
}

void OutputQuoted(const std::string& message) {
  OutputString("\"", kLabelLike);
  OutputString(message, kLabelLike);
  OutputString("\"", kLabelLike);
}

void OutputDefinition(const Target* target) {
  OutputString(":", kLabelLike);
  OutputString(target->label().name(), kLabelLike);
  OutputString(" (defined at ");
  OutputString(target->user_friendly_location().Describe(false), kLabelLike);
  OutputString(")");
}

void OutputInsertionHint(const std::string& key,
                         const std::string& value,
                         const Target* target) {
  OutputSuggestion("Add ");
  OutputString(key);
  OutputString(" = [ ");
  OutputQuoted(value);
  OutputString(" ] to ");
  OutputDefinition(target);
  OutputString("\n");
}

}  // namespace

// Resolves an input to a list of targets, and whether each are private.
// The input can be:
// * A module name for a target
// * A target label
// * A file path, which attempts to resolve to:
//   * Targets defined in the current toolchain that contain the file
//   * Targets defined in the default toolchain that contain the file
//   * Targets defined in any toolchain that contain the file
std::pair<std::vector<std::pair<const Target*, bool>>, bool>
ResolveSuggestionToTarget(const Settings* settings,
                          const std::vector<const Target*>& all_targets,
                          const Label& current_toolchain,
                          std::string_view input) {
  std::vector<std::pair<const Target*, bool>> results;
  auto sorted = [](auto& vec) {
    std::sort(vec.begin(), vec.end(), [](const auto& lhs, const auto& rhs) {
      return lhs.first->label() < rhs.first->label();
    });
    return vec;
  };
  std::string_view module_name = input;
  bool is_private = false;
  if (module_name.ends_with(kPrivateSuffix)) {
    is_private = true;
    module_name.remove_suffix(kPrivateSuffix.size());
  }

  // 1. Try to resolve as a module name.
  for (const Target* target : all_targets) {
    if (target->module_name() == module_name) {
      results.emplace_back(target, is_private);
    }
  }
  if (!results.empty()) {
    return {sorted(results), true};
  }

  // 2. Try to resolve as a target label.
  if (input.starts_with("//") && input.find(':') != std::string_view::npos) {
    Err err;
    Label want;
    if (!all_targets.empty()) {
      Value input_value(nullptr, std::string(input));
      want = Label::Resolve(
          SourceDir("//"),
          all_targets[0]->settings()->build_settings()->root_path_utf8(),
          current_toolchain, input_value, &err);
    }
    if (!err.has_error()) {
      for (const Target* target : all_targets) {
        if (target->label() == want) {
          // We know each label corresponds to exactly one target, so we don't
          // need to keep going.
          results.emplace_back(target, is_private);
          return {results, true};
        }
      }
    }
  }

  // 3. Try to resolve as a file path.
  SourceFile file = ResolveFilePath(settings->build_settings(), input);
  if (file.is_null()) {
    return {results, false};
  }

  if (!AddToolchainSources(all_targets, &current_toolchain, file, results)) {
    AddToolchainSources(all_targets, nullptr, file, results);
  }
  return {sorted(results), true};
}

bool OutputSuggestions(const std::vector<const Target*>& all_targets,
                       Setup* setup,
                       const std::string& includer_name,
                       const std::string& included_name) {
  auto ResolveSuggestion = [&](const std::string& value) {
    const auto& [targets, ok] = ResolveSuggestionToTarget(
        all_targets[0]->settings(), all_targets,
        setup->loader()->GetDefaultToolchain(), value);
    if (!ok) {
      OutputString("Error: ", TextDecoration::DECORATION_RED);
      if (value.starts_with("//")) {
        OutputString("Could not find target or file ");
        OutputQuoted(value);
      } else {
        OutputString("Unable to find ");
        OutputQuoted(value);
        OutputString(" in either the output or source root directories\n");
      }
    }
    return std::make_pair(targets, ok);
  };
  const auto& [includer_targets, includer_ok] =
      ResolveSuggestion(includer_name);
  if (!includer_ok)
    return false;

  if (includer_targets.empty()) {
    OutputString("Error: ", TextDecoration::DECORATION_RED);
    OutputQuoted(includer_name);
    OutputString(" did not resolve to any targets\n");
    return false;
  }

  if (includer_targets.size() > 1) {
    OutputString("Error: ", TextDecoration::DECORATION_RED);
    OutputQuoted(includer_name);
    OutputString(" resolved to multiple targets\n");
    for (const auto& [target, is_private] : includer_targets) {
      OutputString("* ");
      OutputString(target->label().GetUserVisibleName(false), kLabelLike);
      OutputString("\n");
    }
    return true;
  }
  const auto& [includer, source_private] = includer_targets[0];
  const char* dep_field = source_private ? "deps" : "public_deps";
  const Label& current_toolchain = includer->label().GetToolchainLabel();

  auto OutputTarget = [&current_toolchain](const Target* target) {
    OutputString(target->label().GetUserVisibleName(current_toolchain),
                 kLabelLike);
  };
  const auto& [targets, ok] = ResolveSuggestion(included_name);
  if (!ok)
    return false;

  if (targets.empty()) {
    OutputQuoted(included_name);
    OutputString(" is not in the headers of any targets.\n");
    OutputSuggestion("Add ");
    OutputQuoted(included_name);
    OutputString(" to a target's public headers");
    return true;
  }

  std::set<Label> labels_without_toolchain;
  for (const auto& [target, is_private] : targets) {
    labels_without_toolchain.insert(target->label().GetWithNoToolchain());
  }
  if (labels_without_toolchain.size() == 1 &&
      targets[0].first->label().GetToolchainLabel() != current_toolchain) {
    OutputQuoted(included_name);
    OutputString(" is defined in ");
    OutputString(labels_without_toolchain.begin()->GetUserVisibleName(false),
                 kLabelLike);
    OutputString(", but not in the toolchain ");
    OutputString(current_toolchain.GetUserVisibleName(false), kLabelLike);
    OutputString("\n");
    OutputInsertionHint("public", included_name, targets[0].first);
    return true;
  }

  if (targets.size() > 1) {
    OutputWarning();
    OutputQuoted(included_name);
    OutputString("\" is ambiguous because it belongs to multiple targets:\n");
    for (const auto& [target, is_private] : targets) {
      OutputString("* ");
      OutputTarget(target);
      OutputString("\n");
    }
    OutputSuggestion(
        "Create a source_set target for the common headers and sources and "
        "have all of the above targets depend on that.");
    OutputInsertionHint(dep_field, "$NEW_SOURCE_SET", includer);
    return true;
  }

  const auto& [included, included_is_private] = targets[0];
  if (included_is_private) {
    OutputWarning();
    OutputQuoted(included_name);
    OutputString(" is in the private API of ");
    OutputTarget(included);
    OutputSuggestion("Move ");
    OutputQuoted(included_name);
    OutputString(" from `sources` to `public` in ");
    OutputDefinition(included);
  }

  // Note: if we have a toolchain mismatch, we already returned, so the
  // toolchains must match.
  OutputInsertionHint(
      dep_field,
      included->label().dir() == includer->label().dir()
          ? ":" + included->label().name()
          : included->label().GetUserVisibleName(current_toolchain),
      includer);
  return true;
}

int RunSuggest(const std::vector<std::string>& args) {
  if (args.size() <= 1) {
    OutputString("Error: Unknown command format. See \"gn help suggest\"");
    return 1;
  }

  // Deliberately leaked to avoid expensive process teardown.
  Setup* setup = new Setup;
  if (!setup->DoSetup(args[0], false) || !setup->Run())
    return 1;

  std::vector<const Target*> all_targets =
      setup->builder().GetAllResolvedTargets();
  if (all_targets.empty()) {
    OutputString("Error: No targets found\n", TextDecoration::DECORATION_RED);
    return 1;
  }

  bool success = true;
  for (size_t i = 1; i < args.size(); i++) {
    if (i != 1) {
      OutputString("\n");
    }
    std::vector<std::string> pair = base::SplitString(
        args[i], "=", base::TRIM_WHITESPACE, base::SPLIT_WANT_ALL);
    if (pair.size() != 2) {
      OutputString("Error: Invalid pair: " + args[i] + "\n",
                   TextDecoration::DECORATION_RED);
      return 1;
    }
    const auto& includer = pair[0];
    const auto& included = pair[1];

    OutputString("Request: ", TextDecoration::DECORATION_MAGENTA);
    OutputQuoted(includer);
    OutputString(" wants to depend on ");
    OutputQuoted(included);
    OutputString(":\n");

    success &= OutputSuggestions(all_targets, setup, includer, included);
  }

  return success ? 0 : 1;
}

}  // namespace commands
