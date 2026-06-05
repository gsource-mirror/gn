// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ffi/cxx_api.h"
#include "gn/test_with_scope.h"

#include "base/files/file_path.h"
#include "gn/build_settings.h"

// TODO: consider changing this file to be implemented in rust



TestWithScope* NewTestWithScope(rust::Str root_path, rust::Str build_dir) {
  auto* setup = new TestWithScope();
  setup->build_settings()->SetRootPath(base::FilePath(std::string(root_path.data(), root_path.size())));
  setup->build_settings()->SetBuildDir(SourceDir(std::string(build_dir.data(), build_dir.size())));
  return setup;
}

void FreeTestWithScope(TestWithScope* setup) {
  delete setup;
}

Scope* GetScopeFromTestWithScope(TestWithScope& setup) {
  return setup.scope();
}
