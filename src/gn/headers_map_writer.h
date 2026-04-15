// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_HEADERS_MAP_WRITER_H_
#define TOOLS_GN_HEADERS_MAP_WRITER_H_

#include <map>
#include <string_view>
#include <vector>

#include "gn/label.h"
#include "gn/string_output_buffer.h"

class Target;

class HeadersMapWriter {
 public:
  static StringOutputBuffer RunAndGenerate(
      const std::vector<const Target*>& targets);

  static StringOutputBuffer GenerateFiles(
      std::map<std::string_view, std::vector<const Label*>>& header_to_targets);
};

#endif  // TOOLS_GN_HEADERS_MAP_WRITER_H_
