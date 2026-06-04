// Copyright (c) 2013 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_VALUE_ENUM_H_
#define TOOLS_GN_VALUE_ENUM_H_

enum class ValueKind {
  NONE = 0,
  BOOLEAN = 1,
  INTEGER = 2,
  STRING = 3,
  LIST = 4,
  SCOPE = 5,
  STARLARK_VALUE = 6,
};

#endif  // TOOLS_GN_VALUE_ENUM_H_
