// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/settings.h"

#include "gn/label.h"

extern "C" {

const Label& GetToolchainLabelFromSettings(const Settings& settings) {
  return settings.toolchain_label();
}

}  // extern "C"
