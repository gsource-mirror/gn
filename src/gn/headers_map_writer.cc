// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/headers_map_writer.h"

#include <algorithm>
#include <map>
#include <string_view>
#include <vector>

#include "gn/settings.h"
#include "gn/string_output_buffer.h"
#include "gn/target.h"

// static
StringOutputBuffer HeadersMapWriter::GenerateFiles(
    const Label& default_toolchain,
    std::map<std::string_view, std::vector<const Label*>>& header_to_targets) {
  StringOutputBuffer out;
  for (auto& [header_path, targets] : header_to_targets) {
    out.Append(header_path);

    // Unless the user knows exactly what they're doing, they shouldn't be
    // including across a toolchain boundary. So while it's technically
    // allowed, we should never recommend it.
    // Thus, we:
    // * Sort and strip duplicates by the name without the toolchain label.
    // * Strip the toolchain label when printing.
    std::sort(targets.begin(), targets.end(),
              [](const Label* lhs, const Label* rhs) {
                return std::make_tuple(lhs->dir(), lhs->name()) <
                       std::make_tuple(rhs->dir(), rhs->name());
              });
    auto last_unique = std::unique(
        targets.begin(), targets.end(), [](const Label* lhs, const Label* rhs) {
          return lhs->dir() == rhs->dir() && lhs->name() == rhs->name();
        });

    for (auto it = targets.begin(); it != last_unique; ++it) {
      out.Append(" ");
      out.Append((*it)->GetUserVisibleName(false));
    }
    out.Append("\n");
  }

  return out;
}

// static
StringOutputBuffer HeadersMapWriter::RunAndGenerate(
    const std::vector<const Target*>& targets) {
  std::map<std::string_view, std::vector<const Label*>> header_to_targets;
  if (targets.empty()) {
    return {};
  }
  const Label& default_toolchain =
      targets[0]->settings()->default_toolchain_label();

  for (const auto* target : targets) {
    auto process_file = [&](const SourceFile& file) {
      if (file.GetType() == SourceFile::SOURCE_H) {
        std::string_view header_path = file.value();
        if (header_path.rfind("//", 0) == 0) {
          header_path = header_path.substr(2);
        }
        header_to_targets[header_path].push_back(&target->label());
      }
    };

    if (target->all_headers_public()) {
      for (const auto& file : target->sources()) {
        process_file(file);
      }
    } else {
      for (const auto& file : target->public_headers()) {
        process_file(file);
      }
    }
  }

  return GenerateFiles(default_toolchain, header_to_targets);
}
