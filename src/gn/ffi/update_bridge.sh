#!/bin/bash
# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

set -euo pipefail

# Resolve the directory containing this script.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cxxbridge "${SCRIPT_DIR}/../starlark/crates/ffi/src/bridge.rs" --header > "${SCRIPT_DIR}/bridge.h"
cxxbridge "${SCRIPT_DIR}/../starlark/crates/ffi/src/bridge.rs" > "${SCRIPT_DIR}/bridge.cc"
