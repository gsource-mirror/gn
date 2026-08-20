#!/usr/bin/env python3
# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.
"""compare_traces.py - Performance Impact Analysis & Trace Differ for GN.

Loads Chrome trace JSON logs from 'before' and 'after' GN runs, averages
multi-run metrics to eliminate thread jitter, discards false positives using
False Discovery Rate (FDR) testing, and surfaces only the parts of GN that
were legitimately affected (ranked by magnitude and certainty).
"""

import argparse
import collections
import dataclasses
import json
import math
import pathlib
import statistics
from typing import Dict, List, Optional, Tuple


class Stat:
  """Maintains sample statistics (mean, sample stddev, variance) for a metric."""

  def __init__(self, values: List[float]):
    self.values = values
    self.n = len(values)
    self.mean = statistics.mean(values) if values else 0.0
    self.variance = statistics.variance(values) if self.n > 1 else 0.0
    self.stddev = statistics.stdev(values) if self.n > 1 else 0.0

  def format_int(self, show_stddev: bool = True) -> str:
    """Formats mean and standard deviation rounded to integer milliseconds."""
    if self.n > 1 and show_stddev and round(self.stddev) > 0:
      return f'{round(self.mean):,d} ± {round(self.stddev):,d}'
    return f'{round(self.mean):,d}'


def _betacf(a: float, b: float, x: float, max_iter: int = 200) -> float:
  """Continued fraction for regularized incomplete beta function (Lentz method)."""
  qab = a + b
  qap = a + 1.0
  qam = a - 1.0
  c, d = 1.0, 1.0 - qab * x / qap
  if abs(d) < 1e-30:
    d = 1e-30
  d = 1.0 / d
  h = d
  for m in range(1, max_iter):
    m2 = 2 * m
    aa = m * (b - m) * x / ((qam + m2) * (a + m2))
    d = 1.0 + aa * d
    if abs(d) < 1e-30:
      d = 1e-30
    c = 1.0 + aa / c
    if abs(c) < 1e-30:
      c = 1e-30
    d = 1.0 / d
    h *= d * c
    aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2))
    d = 1.0 + aa * d
    if abs(d) < 1e-30:
      d = 1e-30
    c = 1.0 + aa / c
    if abs(c) < 1e-30:
      c = 1e-30
    d = 1.0 / d
    del_h = d * c
    h *= del_h
    if abs(del_h - 1.0) < 1e-12:
      break
  return h


def betainc(a: float, b: float, x: float) -> float:
  """Regularized incomplete beta function I_x(a, b)."""
  if x <= 0:
    return 0.0
  if x >= 1:
    return 1.0
  front = math.exp(
      math.lgamma(a + b)
      - math.lgamma(a)
      - math.lgamma(b)
      + a * math.log(x)
      + b * math.log(1.0 - x)
  )
  if x < (a + 1.0) / (a + b + 2.0):
    return front * _betacf(a, b, x) / a
  else:
    return 1.0 - front * _betacf(b, a, 1.0 - x) / b


def compute_p_value(b_stat: Stat, a_stat: Stat) -> Optional[float]:
  """Computes two-tailed p-value using Welch's t-test with Welch-Satterthwaite df."""
  if b_stat.n < 2 or a_stat.n < 2:
    return None

  v1 = b_stat.variance / b_stat.n
  v2 = a_stat.variance / a_stat.n
  denom = math.sqrt(v1 + v2)
  if denom == 0:
    return 1.0 if b_stat.mean == a_stat.mean else 0.0

  t = (a_stat.mean - b_stat.mean) / denom
  df_num = (v1 + v2) ** 2
  df_den = (v1**2) / (b_stat.n - 1) + (v2**2) / (a_stat.n - 1)
  df = df_num / df_den if df_den > 0 else 1.0

  x = df / (df + t * t)
  return betainc(0.5 * df, 0.5, x)


def benjamini_hochberg_adjust(p_values: List[float]) -> List[float]:
  """Controls False Discovery Rate (FDR) across thousands of parallel comparisons.

  When evaluating thousands of files, a standard alpha=0.05 cutoff produces ~5%
  false positives by pure chance. The Benjamini-Hochberg procedure adjusts
  p-values
  so the global expected false discovery rate remains below alpha.
  """
  m = len(p_values)
  if m == 0:
    return []

  indexed_p = sorted(enumerate(p_values), key=lambda x: x[1])
  adjusted = [0.0] * m

  running_min = 1.0
  for rank_rev, (orig_idx, p) in enumerate(reversed(indexed_p)):
    rank = m - rank_rev
    adj_p = min(1.0, (m / rank) * p)
    running_min = min(running_min, adj_p)
    adjusted[orig_idx] = running_min

  return adjusted


def compute_relevance_score(delta_ms: float, p_val: Optional[float]) -> float:
  """Ranks changes by actionability: magnitude * certainty * regression priority.

  Regressions receive a 1.5x weight over speedups so problematic files appear
  at the top of the report.
  """
  abs_delta = abs(delta_ms)
  direction_weight = 1.5 if delta_ms > 0 else 1.0

  if p_val is not None and p_val > 0:
    # Scale certainty by -log10(p): p=0.001 -> 3.0, p=0.01 -> 2.0, p=0.05 -> 1.3
    certainty = -math.log10(max(p_val, 1e-4))
  else:
    certainty = 1.0

  return abs_delta * certainty * direction_weight


# ============================================================================
# Trace Parsing & Multi-Run Aggregation
# ============================================================================

CATEGORY_DESCRIPTIONS = {
    'file_write_ninja': 'Ninja File Emission',
    'file_exec_template': 'Template Invocations',
    'file_exec': 'BUILD.gn Execution',
    'define': 'Target Definitions',
    'onresolved': 'Target Resolution',
    'import_load': 'Import Loading',
    'script_exec': 'External Scripts',
    'parse': 'AST Parsing',
    'load': 'File Loading',
    'import_block': 'Import Block',
    'file_write_generated': 'Generated File Emission',
    'walk_metadata': 'Metadata Walks',
    'setup': 'Initialization & Setup',
}


def load_trace_events(trace_path: pathlib.Path) -> List[dict]:
  """Reads Chrome trace JSON file and returns the list of traceEvents."""
  with open(trace_path, 'r', encoding='utf-8') as f:
    data = json.load(f)
  return data.get('traceEvents', [])


def extract_single_trace_metrics(events: List[dict]) -> dict:
  """Processes raw events for one run into categories, subsystems, and files."""
  categories = collections.defaultdict(lambda: {'dur_us': 0, 'count': 0})
  file_execs = collections.defaultdict(lambda: {'dur_us': 0, 'count': 0})
  file_parses = collections.defaultdict(lambda: {'dur_us': 0, 'count': 0})
  subsystems = collections.defaultdict(lambda: {'dur_us': 0, 'count': 0})
  script_execs = collections.defaultdict(lambda: {'dur_us': 0, 'count': 0})

  min_ts, max_ts, total_events = None, None, 0

  for event in events:
    # Only complete events ('ph': 'X') represent measured function execution
    if event.get('ph') != 'X':
      continue

    total_events += 1
    cat = event.get('cat', 'unknown')
    dur = event.get('dur', 0)
    ts = event.get('ts', 0)
    name = event.get('name', '')

    if min_ts is None or ts < min_ts:
      min_ts = ts
    if max_ts is None or (ts + dur) > max_ts:
      max_ts = ts + dur

    categories[cat]['dur_us'] += dur
    categories[cat]['count'] += 1

    if cat in ('file_exec', 'parse'):
      # Group by top-level subsystem directory (e.g. //third_party/blink)
      parts = name.split('/')
      if name.startswith('//'):
        subsystem = (
            '//' + '/'.join(parts[2:4])
            if len(parts) > 3
            else ('//' + parts[2] if len(parts) > 2 else name)
        )
      else:
        subsystem = '/'.join(parts[:2]) if len(parts) > 1 else name
      subsystems[subsystem]['dur_us'] += dur
      subsystems[subsystem]['count'] += 1

    if cat == 'file_exec':
      file_execs[name]['dur_us'] += dur
      file_execs[name]['count'] += 1
    elif cat == 'parse':
      file_parses[name]['dur_us'] += dur
      file_parses[name]['count'] += 1
    elif cat == 'script_exec':
      script_execs[name]['dur_us'] += dur
      script_execs[name]['count'] += 1

  wall_clock_ms = (
      ((max_ts - min_ts) / 1000.0)
      if (min_ts is not None and max_ts is not None)
      else 0.0
  )
  total_cpu_ms = sum(c['dur_us'] for c in categories.values()) / 1000.0

  return {
      'wall_clock_ms': wall_clock_ms,
      'total_cpu_ms': total_cpu_ms,
      'total_events': total_events,
      'categories': categories,
      'subsystems': subsystems,
      'file_execs': file_execs,
      'file_parses': file_parses,
      'script_execs': script_execs,
  }


def _aggregate_metric_map(
    runs: List[dict], field_name: str
) -> Dict[str, Dict[str, Stat]]:
  """Aggregates a metric map (categories, files, etc.) across multiple runs into Stats."""
  all_keys = set()
  for r in runs:
    all_keys.update(r[field_name].keys())

  aggregated = {}
  for key in all_keys:
    durs = [r[field_name].get(key, {}).get('dur_us', 0) / 1000.0 for r in runs]
    counts = [r[field_name].get(key, {}).get('count', 0) for r in runs]
    aggregated[key] = {
        'duration_ms': Stat(durs),
        'count': Stat(counts),
    }
  return aggregated


def aggregate_multiple_traces(trace_paths: List[pathlib.Path]) -> dict:
  """Loads multiple trace files for a revision and returns averaged Stats."""
  runs = [
      extract_single_trace_metrics(load_trace_events(p)) for p in trace_paths
  ]

  return {
      'run_count': len(runs),
      'wall_clock': Stat([r['wall_clock_ms'] for r in runs]),
      'cpu_time': Stat([r['total_cpu_ms'] for r in runs]),
      'events': Stat([r['total_events'] for r in runs]),
      'categories': _aggregate_metric_map(runs, 'categories'),
      'subsystems': _aggregate_metric_map(runs, 'subsystems'),
      'file_execs': _aggregate_metric_map(runs, 'file_execs'),
      'script_execs': _aggregate_metric_map(runs, 'script_execs'),
  }


# ============================================================================
# Significant Change Detection
# ============================================================================


@dataclasses.dataclass
class ChangedItem:
  name: str
  before_ms: float
  after_ms: float
  delta_ms: float
  delta_pct: float
  p_val: Optional[float]
  relevance_score: float


def find_significant_changes(
    b_map: Dict[str, Dict[str, Stat]],
    a_map: Dict[str, Dict[str, Stat]],
    min_delta_ms: float,
    alpha: float,
    has_multi_run: bool,
) -> List[ChangedItem]:
  """Identifies items with significant deltas, applies FDR correction, and ranks by relevance."""
  all_keys = set(b_map.keys()) | set(a_map.keys())
  candidates = []
  raw_p_values = []

  for key in all_keys:
    b_stat = b_map.get(key, {}).get('duration_ms', Stat([0.0]))
    a_stat = a_map.get(key, {}).get('duration_ms', Stat([0.0]))
    d_ms = a_stat.mean - b_stat.mean

    if abs(d_ms) >= min_delta_ms:
      p_val = compute_p_value(b_stat, a_stat) if has_multi_run else None
      candidates.append((key, b_stat.mean, a_stat.mean, d_ms, p_val))
      if has_multi_run and p_val is not None:
        raw_p_values.append(p_val)

  significant_items = []
  if has_multi_run and raw_p_values:
    adjusted_p_values = benjamini_hochberg_adjust(raw_p_values)
    for (key, b_ms, a_ms, d_ms, _), adj_p in zip(candidates, adjusted_p_values):
      if adj_p < alpha:
        pct = (d_ms / b_ms * 100.0) if b_ms > 0 else 0.0
        score = compute_relevance_score(d_ms, adj_p)
        significant_items.append(
            ChangedItem(key, b_ms, a_ms, d_ms, pct, adj_p, score)
        )
  elif not has_multi_run:
    for key, b_ms, a_ms, d_ms, _ in candidates:
      pct = (d_ms / b_ms * 100.0) if b_ms > 0 else 0.0
      score = compute_relevance_score(d_ms, None)
      significant_items.append(
          ChangedItem(key, b_ms, a_ms, d_ms, pct, None, score)
      )

  # Rank by Relevance Score (regressions first, then largest speedups)
  significant_items.sort(key=lambda item: item.relevance_score, reverse=True)
  return significant_items


# ============================================================================
# Report Formatting & Presentation
# ============================================================================


def print_executive_verdict(b: dict, a: dict, alpha: float) -> None:
  """Prints high-level conclusion on whether performance changed."""
  wall_delta = a['wall_clock'].mean - b['wall_clock'].mean
  wall_pct = (
      (wall_delta / b['wall_clock'].mean * 100.0)
      if b['wall_clock'].mean > 0
      else 0.0
  )
  wall_p = compute_p_value(b['wall_clock'], a['wall_clock'])
  has_multi_run = b['run_count'] > 1 and a['run_count'] > 1

  print('\nEXECUTIVE VERDICT')
  print('-' * 86)
  if not has_multi_run:
    sign = '+' if round(wall_delta) > 0 else ''
    print(
        '  Single-run comparison: Wall-clock delta'
        f' {sign}{round(wall_delta):,d} ms ({sign}{wall_pct:.1f}%)'
    )
    print(
        '  (Tip: Pass multiple --before and --after runs for statistical'
        ' confidence testing)'
    )
  elif wall_p is not None and wall_p < alpha and abs(wall_pct) >= 1.0:
    if wall_delta > 0:
      print(
          f'  ⚠️  REGRESSION DETECTED: Slower by +{round(wall_delta):,d} ms'
          f' (+{wall_pct:.1f}%)'
      )
    else:
      print(
          f'  🚀 SPEEDUP DETECTED: Faster by -{abs(round(wall_delta)):,d} ms'
          f' ({wall_pct:.1f}%)'
      )
  else:
    print(
        '  ✓ NO MEASURABLE OVERALL IMPACT DETECTED (Wall-clock delta'
        f' {round(wall_delta):+d} ms / {wall_pct:+.1f}%)'
    )

  print(
      f'\n  • Wall-clock / run:  Before: {b["wall_clock"].format_int():>10} ms '
      f' ->  After: {a["wall_clock"].format_int():>10} ms'
  )
  print(
      f'  • Total CPU / run:   Before: {b["cpu_time"].format_int():>10} ms  -> '
      f' After: {a["cpu_time"].format_int():>10} ms'
  )
  print(
      f'  • Events / run:      Before: {round(b["events"].mean):>10,d}     -> '
      f' After: {round(a["events"].mean):>10,d}    (Δ'
      f' {round(a["events"].mean - b["events"].mean):+d})'
  )
  print()


def print_changed_items_table(
    item_type: str,
    items: List[ChangedItem],
    max_items: int = 15,
) -> None:
  """Prints a ranked table of changed items, or a clean confirmation if empty."""
  print(f'AFFECTED {item_type.upper()} (Ranked by impact)')
  print('-' * 86)
  if items:
    print(
        f'{"Delta (ms)":>11} {"Delta (%)":>10} {"Before":>11} {"After":>11}'
        f'  {"Name"}'
    )
    print('-' * 86)
    for item in items[:max_items]:
      sign = '+' if round(item.delta_ms) > 0 else ''
      pct_sign = '+' if item.delta_pct > 0 else ''
      print(
          f'{sign + f"{round(item.delta_ms):,d} ms":>11}'
          f' {pct_sign + f"{item.delta_pct:.1f}%":>9}'
          f' {round(item.before_ms):>8,d} ms {round(item.after_ms):>8,d} ms '
          f' {item.name}'
      )
  else:
    print(
        f'  ✓ Discarded all noise: No {item_type} had statistically significant'
        ' regressions or speedups.'
    )
  print()


def analyze_and_report(
    b: dict, a: dict, min_delta_ms: float = 5.0, alpha: float = 0.05
) -> None:
  """Main coordinator: evaluates traces and produces the final cleaned report."""
  has_multi_run = b['run_count'] > 1 and a['run_count'] > 1

  print('=' * 86)
  print(' GN PERFORMANCE IMPACT REPORT')
  print('=' * 86)
  print(
      f'Samples:  Before: {b["run_count"]} run(s) | After: {a["run_count"]}'
      ' run(s)'
  )

  # 1. Executive Verdict
  print_executive_verdict(b, a, alpha)

  # 2. GN Phase Breakdown
  phase_changes = find_significant_changes(
      b['categories'], a['categories'], min_delta_ms, alpha, has_multi_run
  )
  for item in phase_changes:
    item.name = CATEGORY_DESCRIPTIONS.get(item.name, item.name)
  print_changed_items_table('GN phases', phase_changes)

  # 3. Affected Subsystems Drill-down
  subsystem_changes = find_significant_changes(
      b['subsystems'], a['subsystems'], min_delta_ms, alpha, has_multi_run
  )
  print_changed_items_table('subsystems', subsystem_changes, max_items=10)

  # 4. Affected BUILD Files & Scripts Drill-down
  file_changes = find_significant_changes(
      b['file_execs'], a['file_execs'], min_delta_ms, alpha, has_multi_run
  )
  print_changed_items_table('BUILD files', file_changes, max_items=15)


# ============================================================================
# CLI Entry Point
# ============================================================================


def main() -> None:
  parser = argparse.ArgumentParser(
      description=(
          'GN Trace Impact Report: Discards noise, ranks by relevance, and'
          ' highlights affected parts.'
      ),
      formatter_class=argparse.RawDescriptionHelpFormatter,
      epilog="""Examples:
  compare_traces.py --before 1.trace 2.trace 3.trace --after 4.trace 5.trace 6.trace
  compare_traces.py --before before.trace --after after.trace
""",
  )
  parser.add_argument(
      '--before',
      nargs='+',
      action='extend',
      type=pathlib.Path,
      required=True,
      help='One or more "before" trace JSON files.',
  )
  parser.add_argument(
      '--after',
      nargs='+',
      action='extend',
      type=pathlib.Path,
      required=True,
      help='One or more "after" trace JSON files.',
  )
  parser.add_argument(
      '--min-delta',
      type=float,
      default=5.0,
      help='Minimum effect size in ms to consider (default: 5.0).',
  )
  parser.add_argument(
      '--alpha',
      type=float,
      default=0.05,
      help='Significance threshold FDR Q-value (default: 0.05).',
  )
  args = parser.parse_args()

  b = aggregate_multiple_traces(args.before)
  a = aggregate_multiple_traces(args.after)
  analyze_and_report(b, a, min_delta_ms=args.min_delta, alpha=args.alpha)


if __name__ == '__main__':
  main()
