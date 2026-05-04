// Copyright 2014 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/output_file.h"

#include "gn/filesystem_utils.h"
#include "gn/source_file.h"
#include "gn/build_settings.h"
#include "base/files/file_util.h"

OutputFile::OutputFile(std::string&& v) : value_(std::move(v)) {}

OutputFile::OutputFile(const std::string& v) : value_(v) {}

OutputFile::OutputFile(const BuildSettings* build_settings,
                       const SourceFile& source_file) {
  std::string path_to_rebase = source_file.value();
  if (build_settings && !build_settings->secondary_source_path().empty()) {
    base::FilePath primary_path = build_settings->GetFullPath(source_file);
    if (!base::PathExists(primary_path)) {
      base::FilePath secondary_path = build_settings->GetFullPathSecondary(source_file);
      if (base::PathExists(secondary_path)) {
        std::string abs_path_str = FilePathToUTF8(secondary_path);
#if defined(OS_WIN)
        if (abs_path_str.size() > 1 && abs_path_str[1] == ':') {
          abs_path_str = "/" + abs_path_str;
        }
#endif
        path_to_rebase = abs_path_str;
      }
    }
  }

  value_ = RebasePath(path_to_rebase,
                      build_settings->build_dir(),
                      build_settings->root_path_utf8());
}

SourceFile OutputFile::AsSourceFile(const BuildSettings* build_settings) const {
  DCHECK(!value_.empty());
  DCHECK(value_[value_.size() - 1] != '/');

  std::string path = build_settings->build_dir().value();
  path.append(value_);
  return SourceFile(std::move(path));
}

SourceDir OutputFile::AsSourceDir(const BuildSettings* build_settings) const {
  if (!value_.empty()) {
    // Empty means the root build dir. Otherwise, we expect it to end in a
    // slash.
    DCHECK(value_[value_.size() - 1] == '/');
  }
  std::string path = build_settings->build_dir().value();
  path.append(value_);
  NormalizePath(&path);
  return SourceDir(std::move(path));
}
