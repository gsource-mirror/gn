// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_CHECK_INCLUDES_STRICT_EXCEPTIONS_H_
#define TOOLS_GN_CHECK_INCLUDES_STRICT_EXCEPTIONS_H_

#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

#include "gn/err.h"
#include "gn/label.h"
#include "gn/label_pattern.h"
#include "gn/source_dir.h"

class InputFile;

// CheckIncludesStrictExceptions is conceptually, a list of pairs of
// LabelPattern. However, it is not represented as such internally for
// performance reasons.
class CheckIncludesStrictExceptions {
 public:
  static Result<CheckIncludesStrictExceptions> Parse(
      const InputFile& input_file);

  void AddException(const LabelPattern& from, const LabelPattern& to);

  // Checks whether AddException was ever called with a (from_pattern,
  // to_pattern) such that from_pattern.Matches(from) && to_pattern.Matches(to)
  bool CanInclude(const Label& from, const Label& to) const;

 private:
  struct ToPatterns {
    void Add(const LabelPattern& to);
    bool Matches(const Label& to) const;

    struct DirRule {
      bool recursive = false;
      bool all_in_dir = false;
      std::vector<std::string> exact_targets;
    };
    std::unordered_map<SourceDir, DirRule> dir_rules;
  };

  struct FromDirRules {
    ToPatterns recursive;
    ToPatterns all_in_dir;
    std::unordered_map<std::string, ToPatterns> exact_targets;
  };

  std::unordered_map<SourceDir, FromDirRules> from_rules_;
};

#endif  // TOOLS_GN_CHECK_INCLUDES_STRICT_EXCEPTIONS_H_
