// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "cxx.h"
#include "gn/label.h"

extern "C" {

rust::Str GetLabelDir(const Label& label) {
  return rust::Str(label.dir().value().data(), label.dir().value().size());
}

rust::Str GetLabelName(const Label& label) {
  return rust::Str(label.name().data(), label.name().size());
}

} // extern "C"
