// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_CHECK_INCLUDES_STRICT_EXCEPTIONS_H_
#define TOOLS_GN_CHECK_INCLUDES_STRICT_EXCEPTIONS_H_

#include <memory>
#include <string>
#include <string_view>
#include <unordered_map>
#include <unordered_set>

#include "gn/err.h"
#include "gn/label.h"
#include "gn/label_pattern.h"
#include "gn/source_dir.h"

class Value;

// StringViewHash allows heterogeneous string_view lookups in std::unordered_map
// and std::unordered_set without allocating temporary std::string objects.
struct StringViewHash {
  using is_transparent = void;
  size_t operator()(std::string_view sv) const {
    return std::hash<std::string_view>()(sv);
  }
};

// Represents a set of LabelPatterns stored in a trie structure for fast
// hierarchical lookup. Toolchains are not matched or supported by this
// structure.
//
// Matching against a collection of patterns takes O(directory depth) rather
// than O(number of patterns), with short-circuiting on recursive directory
// wildcards.
class LabelPatterns {
 public:
  // Adds a pattern to the trie.
  void AddPattern(const LabelPattern& pattern);

  // Returns true if any pattern in this collection matches the given label.
  bool Matches(const Label& label) const;

 private:
  struct Node {
    // Matches this directory and all descendant directories (from
    // RECURSIVE_DIRECTORY).
    bool is_recursive = false;

    // Matches all targets directly in this directory (from DIRECTORY).
    bool is_directory = false;

    // Exact target names in this directory (from MATCH).
    std::unordered_set<std::string, StringViewHash, std::equal_to<>> targets;

    // Child directory components (e.g. "foo", "bar").
    std::unordered_map<std::string,
                       std::unique_ptr<Node>,
                       StringViewHash,
                       std::equal_to<>>
        children;
  };

  Node root_;
};

// CheckIncludesStrictExceptions stores allowlist rules for
// check_includes_strict. An includer LabelPattern can be allowed to include
// from any target (global) or from a specific target Label.
class CheckIncludesStrictExceptions {
 public:
  static Result<CheckIncludesStrictExceptions> FromValue(
      const Value& value,
      const SourceDir& current_dir,
      std::string_view source_root);

  void AddGlobalException(const LabelPattern& from) {
    empty_ = false;
    global_.AddPattern(from);
  }
  void AddLocalException(const LabelPattern& from, const Label& to) {
    empty_ = false;
    specific_targets_[to.GetWithNoToolchain()].AddPattern(from);
  }

  // Checks whether `from` is allowed to include headers from `to`.
  bool HasException(const Label& from, const Label& to) const;

  // Checks whether `from` matches any global exception rule.
  bool HasGlobalException(const Label& from) const;

  // Checks whether `(from, to)` matches any specific (local) exception rule.
  bool HasLocalException(const Label& from, const Label& to) const;

  // Returns true if there are no exception rules.
  bool empty() const { return empty_; }

 private:
  bool empty_ = true;
  LabelPatterns global_;
  std::unordered_map<Label, LabelPatterns> specific_targets_;
};

#endif  // TOOLS_GN_CHECK_INCLUDES_STRICT_EXCEPTIONS_H_
