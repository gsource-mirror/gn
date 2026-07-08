// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/phony.h"

#include <algorithm>

#include "gn/filesystem_utils.h"
#include "gn/path_output.h"
#include "gn/target.h"

Phony::Phony(std::vector<OutputFile> direct,
             std::vector<std::optional<OutputFile>> transitive,
             const Target* target,
             std::string_view suffix) {
  // Might be slightly overkill in the case of empty transitive depsets.
  children_.reserve(direct.size() + transitive.size());
  children_ = std::move(direct);
  for (const std::optional<OutputFile>& dep : transitive) {
    if (dep) {
      children_.push_back(*dep);
    }
  }

  // Sort for deterministic output.
  std::sort(children_.begin(), children_.end());

  if (children_.size() >= 2) {
    phony_ = GetOutputFile(*target, BuildDirType::PHONY, target->label().name(), suffix);
  } else if (children_.size() == 1) {
    phony_ = children_[0];
  } else {
    phony_ = std::nullopt;
  }
}

void Phony::Write(std::ostream& out, const PathOutput& path_output) const {
  if (phony_ && children_.size() >= 2) {
    out << "build ";
    path_output.WriteFile(out, *phony_);
    out << ": phony";
    for (const auto& child : children_) {
      out << " ";
      path_output.WriteFile(out, child);
    }
    out << '\n';
  }
}
