// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_FFI_SLICE_H_
#define TOOLS_GN_FFI_SLICE_H_

#include <array>
#include <cstdint>
#include <vector>
#include "gn/ffi/bridge.h"
template <typename T>
inline SliceAny IntoSlice(std::vector<T> vec) {
  if (vec.empty()) {
    return SliceAny{0, nullptr};
  }
  SliceAny slice{vec.size(), reinterpret_cast<Any*>(vec.data())};

  // Construct on stack buffer to prevent C++ compiler from running destructor
  // on vec
  std::array<uint8_t, sizeof(std::vector<T>)> buf;
  new (&buf) std::vector<T>(std::move(vec));

  return slice;
}

template <typename T>
inline SliceAny AsSlice(const std::vector<T>& vec) {
  return SliceAny{vec.size(),
                  reinterpret_cast<Any*>(const_cast<T*>(vec.data()))};
}

#endif  // TOOLS_GN_FFI_SLICE_H_
