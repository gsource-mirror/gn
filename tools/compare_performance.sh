#!/usr/bin/env bash
# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

error() {
  echo "$@" >&2
  exit 1
}

if [[ $# -lt 3 ]]; then
  echo "Usage: $0 <out_dir> <revision> <reference_workspace_dir> [hyperfine_args...]" >&2
  echo "Example:" >&2
  echo "  $0 out/Default @- ~/gn_reference" >&2
  exit 1
fi

OUT_DIR="$1"
REV="$2"
REF_DIR="$3"
shift 3
HYPERFINE_EXTRA_ARGS=("$@")

# Verify prerequisites
if ! command -v hyperfine &> /dev/null; then
  error "hyperfine is not installed or not in PATH"
fi

# 1. Build both binaries using double_build.sh
echo "==> Building both GN versions via double_build.sh..."
"${SCRIPT_DIR}/double_build.sh" "${REV}" "${REF_DIR}"

BEFORE_BIN="/tmp/before/gn"
AFTER_BIN="/tmp/after/gn"

if [[ ! -x "${BEFORE_BIN}" ]]; then
  error "Could not find built binary at ${BEFORE_BIN}"
fi
if [[ ! -x "${AFTER_BIN}" ]]; then
  error "Could not find built binary at ${AFTER_BIN}"
fi

# 2. Run Hyperfine benchmark outputting JSON
HYPERFINE_JSON="/tmp/hyperfine_results.json"
echo "==> Running Hyperfine benchmark on 'gn gen ${OUT_DIR}'..."
hyperfine \
  --export-json "${HYPERFINE_JSON}" \
  --warmup 2 \
  ${HYPERFINE_EXTRA_ARGS[@]+"${HYPERFINE_EXTRA_ARGS[@]}"} \
  -n "before (${REV})" "${BEFORE_BIN} gen ${OUT_DIR}" \
  -n "after (@)" "${AFTER_BIN} gen ${OUT_DIR}"

echo "==> Hyperfine results saved to ${HYPERFINE_JSON}"

# 3. Run once with --tracelog on each for fine-grained performance breakdown
TRACE_BEFORE="/tmp/trace_before.json"
TRACE_AFTER="/tmp/trace_after.json"

echo "==> Collecting fine-grained trace logs..."
"${BEFORE_BIN}" gen "${OUT_DIR}" --tracelog="${TRACE_BEFORE}" > /dev/null
"${AFTER_BIN}" gen "${OUT_DIR}" --tracelog="${TRACE_AFTER}" > /dev/null

# 4. Analyze and display trace log comparison
"${SCRIPT_DIR}/analyze_trace.py" "${TRACE_BEFORE}" "${TRACE_AFTER}"
