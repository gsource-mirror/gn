#!/usr/bin/env bash
# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <revision> <reference_workspace_dir> [--build/gen.py args...]" >&2
  exit 1
fi

REV="$1"
REF_DIR="$(realpath -m "${2/#\~/$HOME}")"
shift 2
GEN_ARGS=("$@")

error() {
  echo "$@" >&2
  exit 1
}

# Ensure we are in the GN repository root.
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Evaluate the target revision to a concrete commit ID in the GN workspace.
COMMIT_ID="$(jj log -r "${REV}" --no-graph -T 'commit_id' | head -n 1)"
if [[ -z "${COMMIT_ID}" ]]; then
  error "Could not resolve revision '${REV}'"
fi

# Set up or update the reference workspace to the target commit.
if [[ ! -d "${REF_DIR}" ]]; then
  error "Could not find reference workspace at ${REF_DIR}"
fi

echo "==> Updating reference workspace in ${REF_DIR} to ${COMMIT_ID}..."
(cd "${REF_DIR}" && jj new "${COMMIT_ID}" && build/gen.py --out-path=/tmp/before "${GEN_ARGS[@]}" && ninja -C /tmp/before gn)
build/gen.py --out-path=/tmp/after "${GEN_ARGS[@]}" && ninja -C /tmp/after gn
