#!/usr/bin/env python3
# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

import argparse
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

COLOR_GREEN = '\033[32m'
COLOR_RED = '\033[31m'
COLOR_CYAN = '\033[36m'
COLOR_BOLD = '\033[1m'
COLOR_RESET = '\033[0m'


def colorize(text: str, color: str) -> str:
  if sys.stdout.isatty():
    return f'{color}{text}{COLOR_RESET}'
  return text


def build_gn_at_revision(
    rev_or_bin: str,
    repo_dir: Path,
    dest_dir: Path,
    debug: bool = False,
) -> Path:
  """Returns path to a GN binary, building it via a jj workspace if a revision is given."""
  # If an existing file path was supplied, use it directly.
  potential_path = Path(rev_or_bin)
  if potential_path.is_file() or (potential_path.exists() and not potential_path.is_dir()):
    return potential_path.resolve()

  rev = rev_or_bin
  sanitized_rev = rev.replace('@', 'at').replace('/', '_').replace(':', '_')
  out_binary = dest_dir / f'gn_{sanitized_rev}'

  print(colorize(f'Building GN at revision "{rev}"...', COLOR_BOLD))

  # If building current working copy (@), we can build directly in the repo.
  if rev == '@':
    build_dir = repo_dir / 'out' / ('Debug' if debug else 'Release')
    gen_cmd = ['python3', 'build/gen.py', '--out-path', str(build_dir)]
    if debug:
      gen_cmd.append('--debug')
    subprocess.run(gen_cmd, cwd=repo_dir, check=True)
    subprocess.run(['ninja', '-C', str(build_dir), 'gn'], cwd=repo_dir, check=True)
    shutil.copy2(build_dir / 'gn', out_binary)
    print(colorize(f'Successfully built GN at revision "{rev}" -> {out_binary}\n', COLOR_GREEN))
    return out_binary

  # For other revisions, build in an isolated temporary jj workspace.
  temp_ws = Path(tempfile.mkdtemp(prefix=f'gn_ws_{sanitized_rev}_'))
  try:
    subprocess.run(
        ['jj', 'workspace', 'add', str(temp_ws), '-r', rev],
        cwd=repo_dir,
        check=True,
    )
    ws_build_dir = temp_ws / 'out' / ('Debug' if debug else 'Release')
    gen_cmd = ['python3', 'build/gen.py', '--out-path', str(ws_build_dir)]
    if debug:
      gen_cmd.append('--debug')
    subprocess.run(gen_cmd, cwd=temp_ws, check=True)
    subprocess.run(['ninja', '-C', str(ws_build_dir), 'gn'], cwd=temp_ws, check=True)
    shutil.copy2(ws_build_dir / 'gn', out_binary)
  finally:
    subprocess.run(['jj', 'workspace', 'forget', str(temp_ws)], cwd=repo_dir, check=True)
    shutil.rmtree(temp_ws, ignore_errors=True)

  print(colorize(f'Successfully built GN at revision "{rev}" -> {out_binary}\n', COLOR_GREEN))
  return out_binary


def main() -> int:
  parser = argparse.ArgumentParser(
      description='Build GN binaries at two different revisions using jj workspaces.'
  )
  parser.add_argument(
      'rev1',
      nargs='?',
      default='@-',
      help='First revision or binary path (default: @-)',
  )
  parser.add_argument(
      'rev2',
      nargs='?',
      default='@',
      help='Second revision or binary path (default: @)',
  )
  parser.add_argument(
      '--repo-dir',
      type=Path,
      default=Path.cwd(),
      help='GN repository directory (default: current directory)',
  )
  parser.add_argument(
      '--debug',
      action='store_true',
      help='Build debug binaries instead of release binaries',
  )
  parser.add_argument(
      '--out-dir',
      type=Path,
      default=None,
      help='Directory where the compiled GN binaries will be saved (temporary directory by default)',
  )

  args = parser.parse_args()
  repo_dir = args.repo_dir.resolve()

  temp_dir = None
  if args.out_dir:
    dest_dir = args.out_dir.resolve()
    dest_dir.mkdir(parents=True, exist_ok=True)
  else:
    temp_dir = tempfile.TemporaryDirectory(prefix='gn_binaries_')
    dest_dir = Path(temp_dir.name)

  bin1 = build_gn_at_revision(args.rev1, repo_dir, dest_dir, debug=args.debug)
  bin2 = build_gn_at_revision(args.rev2, repo_dir, dest_dir, debug=args.debug)

  print(colorize('Both GN binaries compiled successfully:', COLOR_BOLD))
  print(f'  Binary 1 ({args.rev1}): {bin1}')
  print(f'  Binary 2 ({args.rev2}): {bin2}')

  # If a custom output directory was provided, keep the binaries.
  # If a temp dir was used, keep it alive and print its location.
  if temp_dir:
    # Release ownership of temp_dir so files remain accessible after script exits
    temp_dir._finalizer.detach()

  return 0


if __name__ == '__main__':
  sys.exit(main())
