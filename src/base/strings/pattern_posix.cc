// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "base/strings/pattern.h"

#include <fnmatch.h>
#include <string>

#include "base/files/file_path.h"

namespace base {

// Returns true if the path matches the pattern using POSIX fnmatch.
bool FilePathMatchPattern(const base::FilePath& path, base::FilePath::StringViewType pattern) {
  return pattern.empty() ||
    fnmatch(std::string(pattern).c_str(), path.value().c_str(), 0) == 0;
}

}  // namespace base
