// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/check_includes_strict_exceptions.h"

#include <algorithm>
#include <string_view>
#include <vector>

#include "base/strings/string_split.h"
#include "gn/input_file.h"
#include "gn/location.h"
#include "gn/value.h"

namespace {

// Gets the parent directory of a directory.
// Returns the empty string if it is the root directory.
// Returns the parent directory if it is source-absolute.
// Otherwise, returns a string guarunteed to:
// * Not be a key in strict deps exceptions
// * Be shorter than the input
std::string_view GetParentDir(std::string_view dir) {
  if (dir.size() <= 2)
    return {};
  dir.remove_suffix(1);
  size_t last_slash = dir.rfind('/');
  if (last_slash == std::string_view::npos)
    return {};
  return dir.substr(0, last_slash + 1);
}

}  // namespace

void CheckIncludesStrictExceptions::ToPatterns::Add(const LabelPattern& to) {
  auto& rule = dir_rules[to.dir()];
  switch (to.type()) {
    case LabelPattern::RECURSIVE_DIRECTORY:
      rule.recursive = true;
      break;
    case LabelPattern::DIRECTORY:
      rule.all_in_dir = true;
      break;
    case LabelPattern::MATCH:
      rule.exact_targets.push_back(to.name());
      break;
  }
}

bool CheckIncludesStrictExceptions::ToPatterns::Matches(const Label& to) const {
  if (auto it = dir_rules.find(to.dir()); it != dir_rules.end()) {
    const auto& rule = it->second;
    if (rule.recursive || rule.all_in_dir ||
        std::ranges::contains(rule.exact_targets, to.name())) {
      return true;
    }
  }

  for (std::string_view d = GetParentDir(to.dir().value()); !d.empty();
       d = GetParentDir(d)) {
    if (auto it = dir_rules.find(SourceDir(d));
        it != dir_rules.end() && it->second.recursive) {
      return true;
    }
  }

  return false;
}

void CheckIncludesStrictExceptions::AddException(const LabelPattern& from,
                                                 const LabelPattern& to) {
  auto& rules = from_rules_[from.dir()];
  switch (from.type()) {
    case LabelPattern::RECURSIVE_DIRECTORY:
      rules.recursive.Add(to);
      break;
    case LabelPattern::DIRECTORY:
      rules.all_in_dir.Add(to);
      break;
    case LabelPattern::MATCH:
      rules.exact_targets[from.name()].Add(to);
      break;
  }
}

bool CheckIncludesStrictExceptions::CanInclude(const Label& from,
                                               const Label& to) const {
  auto it = from_rules_.find(from.dir());
  if (it != from_rules_.end()) {
    const auto& rules = it->second;
    if (rules.all_in_dir.Matches(to) || rules.recursive.Matches(to))
      return true;
    auto it = rules.exact_targets.find(from.name());
    if (it != rules.exact_targets.end() && it->second.Matches(to)) {
      return true;
    }
  }

  for (std::string_view d = GetParentDir(from.dir().value()); !d.empty();
       d = GetParentDir(d)) {
    if (auto it = from_rules_.find(SourceDir(d));
        it != from_rules_.end() && it->second.recursive.Matches(to)) {
      return true;
    }
  }

  return false;
}

Result<CheckIncludesStrictExceptions> CheckIncludesStrictExceptions::Parse(
    const InputFile& input_file) {
  CheckIncludesStrictExceptions exceptions;
  std::vector<std::string_view> lines = base::SplitStringPiece(
      input_file.contents(), "\n", base::TRIM_WHITESPACE, base::SPLIT_WANT_ALL);

  for (size_t i = 0; i < lines.size(); ++i) {
    std::string_view line = lines[i];
    if (line.empty() || line[0] == '#')
      continue;

    int line_number = static_cast<int>(i + 1);

    auto parse_pattern =
        [&](std::string_view pattern_str) -> Result<LabelPattern> {
      if (!pattern_str.starts_with("//")) {
        return Err(Location(&input_file, line_number, 1),
                   "Label pattern must be absolute (starting with '//').");
      }
      if (pattern_str.find('(') != std::string_view::npos) {
        return Err(Location(&input_file, line_number, 1),
                   "Toolchains are not allowed in exception patterns.");
      }

      Err parse_err;
      Value val(nullptr, std::string(pattern_str));
      LabelPattern pattern = LabelPattern::GetPattern(
          SourceDir("//"), std::string_view(), val, &parse_err);
      if (parse_err.has_error()) {
        return Err(Location(&input_file, line_number, 1), parse_err.message(),
                   parse_err.help_text());
      }

      return pattern;
    };

    std::vector<std::string_view> parts = base::SplitStringPieceUsingSubstr(
        line, "=>", base::TRIM_WHITESPACE, base::SPLIT_WANT_ALL);
    if (parts.size() != 2 || parts[0].empty() || parts[1].empty()) {
      return Err(Location(&input_file, line_number, 1),
                 "Expected 'label_pattern1 => label_pattern2'.");
    }

    ASSIGN_OR_RETURN(LabelPattern from_pattern, parse_pattern(parts[0]));
    ASSIGN_OR_RETURN(LabelPattern to_pattern, parse_pattern(parts[1]));

    exceptions.AddException(from_pattern, to_pattern);
  }

  return exceptions;
}
