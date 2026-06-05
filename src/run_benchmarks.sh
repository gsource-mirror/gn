#!/bin/bash
# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

set -e

CHROMIUM_SRC="/usr/local/google/home/msta/chromium/src"
CSV_OUTPUT="/usr/local/google/home/msta/gn/src/benchmarks.csv"

# Remember the initial state so we can restore it on exit
INITIAL_REV=$(jj show -T "change_id" -r @ -R "$CHROMIUM_SRC" --no-patch)

cd "$CHROMIUM_SRC"

cleanup() {
  echo "Cleaning up..."
  jj new "$INITIAL_REV" > /dev/null 2>&1 || true
  rm -rf "$CHROMIUM_SRC/out/benchmark"
}
trap cleanup EXIT

# Run hyperfine benchmark
# Scenario 1: before binary on clean main@origin
# Scenario 2: after binary on clean main@origin
# Scenario 3: after binary on rule extensions commit (mszzrszx)
# Scenario 4: after binary on header tracking commit (pxvuvsss)
hyperfine \
  --show-output \
  --warmup 2 \
  --runs 5 \
  --export-csv "$CSV_OUTPUT" \
  --prepare "jj new main@origin > /dev/null && rm -rf out/benchmark" \
  -n "Before (main@origin)" \
  "/tmp/gn/before gen out/benchmark" \
  --prepare "jj new main@origin > /dev/null && rm -rf out/benchmark" \
  -n "After (main@origin)" \
  "/tmp/gn/after gen out/benchmark" \
  --prepare "jj new mszzrszx > /dev/null && rm -rf out/benchmark" \
  -n "After (rule_extensions)" \
  "/tmp/gn/after gen out/benchmark" \
  --prepare "jj new pxvuvsss > /dev/null && rm -rf out/benchmark" \
  -n "After (header_tracking)" \
  "/tmp/gn/after gen out/benchmark"

echo "Benchmark complete. Results written to $CSV_OUTPUT."
