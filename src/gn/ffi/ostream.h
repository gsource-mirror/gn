// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_FFI_OSTREAM_H_
#define TOOLS_GN_FFI_OSTREAM_H_

#include <ostream>

#include "cxx.h"

using Ostream = std::ostream;

inline void write_to_ostream(Ostream& os, rust::Str str) {
  os.write(str.data(), str.size());
}

#endif  // TOOLS_GN_FFI_OSTREAM_H_
