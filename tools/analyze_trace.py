#!/usr/bin/env python3
# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

import json
from pathlib import Path
import sys

COLOR_GREEN = '\033[32m'
COLOR_RED = '\033[31m'
COLOR_BOLD = '\033[1m'
COLOR_RESET = '\033[0m'


def colorize(text: str, color: str) -> str:
  if sys.stdout.isatty():
    return f'{color}{text}{COLOR_RESET}'
  return text


def load_trace_events(path: Path) -> list[dict]:
  content = json.loads(path.read_text(encoding='utf-8', errors='replace'))
  if isinstance(content, dict) and 'traceEvents' in content:
    return content['traceEvents']
  if isinstance(content, list):
    return content
  return []


def analyze_traces(trace_before_path: Path, trace_after_path: Path) -> None:
  events_before = load_trace_events(trace_before_path)
  events_after = load_trace_events(trace_after_path)

  # Aggregate category durations in milliseconds
  cat_dur_before: dict[str, float] = {}
  cat_dur_after: dict[str, float] = {}
  cat_count_before: dict[str, int] = {}
  cat_count_after: dict[str, int] = {}

  write_ninja_durs_before: list[float] = []
  write_ninja_durs_after: list[float] = []

  for e in events_before:
    cat = e.get('cat', 'other')
    dur_ms = e.get('dur', 0) / 1000.0
    cat_dur_before[cat] = cat_dur_before.get(cat, 0.0) + dur_ms
    cat_count_before[cat] = cat_count_before.get(cat, 0) + 1
    if cat == 'file_write_ninja':
      write_ninja_durs_before.append(dur_ms)

  for e in events_after:
    cat = e.get('cat', 'other')
    dur_ms = e.get('dur', 0) / 1000.0
    cat_dur_after[cat] = cat_dur_after.get(cat, 0.0) + dur_ms
    cat_count_after[cat] = cat_count_after.get(cat, 0) + 1
    if cat == 'file_write_ninja':
      write_ninja_durs_after.append(dur_ms)

  all_cats = sorted(
      set(cat_dur_before.keys()) | set(cat_dur_after.keys()),
      key=lambda c: max(cat_dur_before.get(c, 0), cat_dur_after.get(c, 0)),
      reverse=True,
  )

  print(colorize('\n=== Tracelog Performance Breakdown (Thread Time) ===\n', COLOR_BOLD))
  header = f'{"Category":<25} {"Before (ms)":>14} {"After (ms)":>14} {"Delta (ms)":>14} {"Change":>10}'
  print(colorize(header, COLOR_BOLD))
  print('-' * len(header))

  for cat in all_cats:
    b = cat_dur_before.get(cat, 0.0)
    a = cat_dur_after.get(cat, 0.0)
    delta = a - b
    pct = (delta / b * 100.0) if b > 0 else 0.0
    sign = '+' if delta > 0 else ''
    pct_str = f'{sign}{pct:.1f}%'
    if delta < -1.0:
      pct_str = colorize(pct_str, COLOR_GREEN)
    elif delta > 1.0:
      pct_str = colorize(pct_str, COLOR_RED)

    print(f'{cat:<25} {b:>14.2f} {a:>14.2f} {delta:>+14.2f} {pct_str:>19}')

  if write_ninja_durs_before and write_ninja_durs_after:
    write_ninja_durs_before.sort()
    write_ninja_durs_after.sort()

    n_b = len(write_ninja_durs_before)
    n_a = len(write_ninja_durs_after)
    med_b = write_ninja_durs_before[n_b // 2]
    med_a = write_ninja_durs_after[n_a // 2]
    p90_b = write_ninja_durs_before[int(n_b * 0.9)]
    p90_a = write_ninja_durs_after[int(n_a * 0.9)]

    print(colorize('\n--- file_write_ninja Detailed Stats ---', COLOR_BOLD))
    print(f'  Target Count:  Before={n_b:,}, After={n_a:,}')
    print(f'  Median Time:   Before={med_b:.3f} ms, After={med_a:.3f} ms ({((med_a - med_b)/med_b*100.0):+.1f}%)')
    print(f'  P90 Time:      Before={p90_b:.3f} ms, After={p90_a:.3f} ms ({((p90_a - p90_b)/p90_b*100.0):+.1f}%)')
    print()


def main() -> int:
  if len(sys.argv) < 3:
    print(f'Usage: {sys.argv[0]} <trace_before.json> <trace_after.json>')
    return 1

  analyze_traces(Path(sys.argv[1]), Path(sys.argv[2]))
  return 0


if __name__ == '__main__':
  sys.exit(main())
