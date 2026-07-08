// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_PHONY_H_
#define TOOLS_GN_PHONY_H_

#include <ostream>
#include <vector>

#include "gn/output_file.h"

class PathOutput;

class Phony {
 public:
  Phony(OutputFile phony, std::vector<OutputFile> children);

  void Write(std::ostream& out, const PathOutput& path_output) const;

 private:
  // The files that this phony expands to
  std::vector<OutputFile> children_;

  // The name of this phony. Only set if children_ is non-empty.
  OutputFile phony_;
};

#endif  // TOOLS_GN_PHONY_H_