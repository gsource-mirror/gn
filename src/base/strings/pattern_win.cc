// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "base/strings/pattern.h"

#include <windows.h>
#include <shlwapi.h>
#include <string>

#include "base/files/file_path.h"

namespace base {

// Returns true if the path matches the pattern using Windows PathMatchSpecW.
bool FilePathMatchPattern(const base::FilePath& path, base::FilePath::StringViewType pattern) {
  if (pattern.empty())
    return true;

  std::wstring wpattern(reinterpret_cast<const wchar_t*>(pattern.data()), pattern.length());
  return PathMatchSpecW(path.value().c_str(), wpattern.c_str()) == TRUE;
}

}  // namespace base
