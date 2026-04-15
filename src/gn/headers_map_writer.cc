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

    std::sort(targets.begin(), targets.end(),
              [](const Label* lhs, const Label* rhs) {
                return std::make_tuple(lhs->dir(), lhs->name()) <
                       std::make_tuple(rhs->dir(), rhs->name());
              });

    auto last_unique = std::unique(
        targets.begin(), targets.end(), [](const Label* lhs, const Label* rhs) {
          return lhs->dir() == rhs->dir() && lhs->name() == rhs->name();
        });
    targets.erase(last_unique, targets.end());

    // Unless the user knows exactly what they're doing, they shouldn't be
    // including across a toolchain boundary. So while it's technically
    // allowed, we should never recommend it.
    // Thus, we strip the toolchain label from the output here.
    for (const Label* label : targets) {
      out.Append(" ");
      out.Append(label->GetUserVisibleName(false));
    }
    out.Append("\n");
  }

  return out;
}

// static
StringOutputBuffer HeadersMapWriter::RunAndGenerate(
    const std::vector<const Target*>& targets) {
  std::map<std::string_view, std::vector<const Label*>> header_to_targets;

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

    for (const auto& file : target->sources()) {
      process_file(file);
    }

    for (const auto& file : target->public_headers()) {
      process_file(file);
    }
  }

  const Label& default_toolchain =
      targets.empty() ? Label()
                      : targets[0]->settings()->default_toolchain_label();
  return GenerateFiles(default_toolchain, header_to_targets);
}
