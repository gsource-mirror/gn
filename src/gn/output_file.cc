// Copyright 2014 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/output_file.h"

#include "gn/filesystem_utils.h"
#include "gn/source_file.h"

OutputFile::OutputFile(const BuildSettings* build_settings,
                       const SourceFile& source_file) {
  std::string rebased =
      RebasePath(source_file.value(), build_settings->build_dir(),
                 build_settings->root_path_utf8());
  value_ = StringAtom(rebased);
}

void OutputFile::resize(std::size_t n) {
  std::string_view v = value();
  if (n < v.size()) {
    value_ = StringAtom(v.substr(0, n));
  } else if (n > v.size()) {
    std::string s(v);
    s.resize(n);
    value_ = StringAtom(s);
  }
}

void OutputFile::append(std::string_view v) {
  std::string s(value_);
  s.append(v);
  value_ = StringAtom(s);
}

SourceFile OutputFile::AsSourceFile(const BuildSettings* build_settings) const {
  std::string_view val = value();
  DCHECK(!val.empty());
  DCHECK(val.back() != '/');

  std::string path = build_settings->build_dir().value();
  path.append(val);
  return SourceFile(std::move(path));
}

SourceDir OutputFile::AsSourceDir(const BuildSettings* build_settings) const {
  std::string_view val = value();
  if (!val.empty()) {
    // Empty means the root build dir. Otherwise, we expect it to end in a
    // slash.
    DCHECK(val.back() == '/');
  }
  std::string path = build_settings->build_dir().value();
  path.append(val);
  NormalizePath(&path);
  return SourceDir(std::move(path));
}
