// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_PHONY_H_
#define TOOLS_GN_PHONY_H_

#include <optional>
#include <string_view>
#include <vector>

#include <ostream>

#include "gn/output_file.h"

class PathOutput;
class Target;

class Phony {
 private:
  Phony(std::vector<OutputFile> direct,
        std::vector<std::optional<OutputFile>> transitive,
        const Target* target,
        std::string_view suffix);

 public:
  // A phony can be one of three states:
  // * empty (children={}, phony=None)
  //   * No phony is generated.
  // * Size 1 (children=[foo], phony=foo)
  //   * No phony is generated; instead, the phony is just the real file.
  // * Size > 1 (children=[foo, bar, ...], phony=generated)
  //   * A phony is generated.
  const std::optional<OutputFile>& phony() const { return phony_; }

  void Write(std::ostream& out, const PathOutput& path_output) const;

 private:
  // The files that this phony expands to
  std::vector<OutputFile> children_;

  // The name of this phony. Only set if children_ is non-empty.
  std::optional<OutputFile> phony_;

  friend class Target;
};

#endif  // TOOLS_GN_PHONY_H_