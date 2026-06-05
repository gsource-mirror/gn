// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/output_file.h"

#include "cxx.h"

extern "C" {

rust::Str GetOutputFilePath(const OutputFile& file) {
  return file.value();
}

}  // extern "C"
