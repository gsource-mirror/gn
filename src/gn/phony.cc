// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/phony.h"
 
#include <algorithm>
 
#include "gn/path_output.h"
 
Phony::Phony(OutputFile phony, std::vector<OutputFile> children)
    : children_(std::move(children)), phony_(std::move(phony)) {
  // Sort for deterministic output.
  std::sort(children_.begin(), children_.end());
}


void Phony::Write(std::ostream& out, const PathOutput& path_output) const {
  out << "build ";
  path_output.WriteFile(out, phony_);
  out << ": phony";
  for (const auto& child : children_) {
    out << " ";
    path_output.WriteFile(out, child);
  }
  out << '\n';
}

