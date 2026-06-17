#!/usr/bin/env python3
# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

import argparse
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys


def get_ninja_files(directory: Path) -> set[Path]:
  ninja_files = set()
  for root, _, files in os.walk(directory):
    for f in files:
      if f.endswith('.ninja'):
        rel = os.path.relpath(os.path.join(root, f), directory)
        ninja_files.add(Path(rel))
  return ninja_files


def main():
  parser = argparse.ArgumentParser(
      description='Compare or update ninja output files.'
  )
  parser.add_argument(
      '--update', action='store_true', help='Update the golden files'
  )
  parser.add_argument('golden_dir', help='Path to golden files directory')
  parser.add_argument('generated_dir', help='Path to generated files directory')
  args = parser.parse_args()

  want_dir = Path(args.golden_dir).resolve()
  got_dir = Path(args.generated_dir).resolve()

  want_files = get_ninja_files(want_dir)
  got_files = get_ninja_files(got_dir)

  return_code = 0
  all_files = want_files | got_files

  for f in sorted(all_files):
    want = want_dir / f
    got = got_dir / f

    if args.update:
      want.unlink(missing_ok=True)
      if f in got_files:
        want.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(got, want)
    elif f not in got_files:
      print(f'Error: Missing generated file: {f}', file=sys.stderr)
      return_code = 1
    elif f not in want_files:
      print(f'Error: Unexpected generated file: {f}', file=sys.stderr)
      return_code = 1
    else:
      res = subprocess.run(
          ['diff', '--color=always', '-u', str(want), str(got)]
      )
      if res.returncode:
        return_code = 1

  if return_code:
    print(
        f'Run `{sys.executable} {Path(__file__).resolve()} {want_dir} {got_dir}'
        ' --update` to update the golden files if the changes are intentional.'
    )
  sys.exit(return_code)


if __name__ == '__main__':
  main()
