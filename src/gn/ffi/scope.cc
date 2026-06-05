// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/scope.h"
#include "gn/settings.h"

extern "C" {

const Settings* GetSettingsFromScope(const Scope& scope) {
  return scope.settings();
}

}  // extern "C"
