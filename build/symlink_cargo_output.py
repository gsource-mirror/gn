#!/usr/bin/env python3
# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

import os
import re
import shutil
import sys
from pathlib import Path

_ESC = '\u001e'

def parse_depfile(src_depfile: Path) -> list[Path]:
    """Parses depfile and returns a flattened list of resolved dependency Paths.
    
    Reads the file, extracts CARGO_MANIFEST_DIR, strips all comment lines,
    cleans up line continuations, and parses and resolves dependencies.
    """
    content = src_depfile.read_text()

    # Strip comments
    content = re.sub(r'(?m)^#.*$', '', content)
    # Strip the output files ("a: b c" -> "b c")
    content = re.sub(r'^[^:]*:\s*', '', content)
    content = content.replace('\\\n', ' ')
    content = content.replace('\\ ', _ESC)
    
    deps = []

    for f in re.split(r'\s+', content.strip()):
        f = Path(f.replace('\\\\', '\\').replace(_ESC, ' '))
        deps.append(f)

    return deps

def to_depfile_path(p: Path, out_dir: Path):
    # Can't use Path.relative_to because we require walk_up=True, which doesn't
    # exist on preinstalled python on macos.
    rel = os.path.relpath(p, out_dir)
    return rel.replace('\\', '\\\\').replace(' ', '\\ ')

def rewrite_depfile(src_depfile: Path, dest_depfile: Path, target_name: str):
    """Orchestrates reading, parsing, and writing the depfile."""
    deps = parse_depfile(src_depfile)
    out_dir = dest_depfile.parent.resolve()
    deps_str = ' '.join(to_depfile_path(d, out_dir) for d in deps)
    new_content = f'{target_name}: {deps_str}\n'
    
    dest_depfile.parent.mkdir(parents=True, exist_ok=True)
    dest_depfile.write_text(new_content)

def main():
    if len(sys.argv) != 4:
        print('Usage: symlink_cargo_output.py <test|lib> <out> <cargo_out_dir>', file=sys.stderr)
        sys.exit(1)

    target_type = sys.argv[1]
    out_path = Path(sys.argv[2])
    cargo_out_dir = Path(sys.argv[3])
    src_depfile = cargo_out_dir / 'libgn_starlark.d'

    if target_type == 'lib':
        # Locate the static library
        src_path = cargo_out_dir / 'libgn_starlark.a'
    else:
        # Locate the newest test binary under deps/
        candidates = [c for c in (cargo_out_dir / 'deps').glob('gn_starlark-*') if not c.name.endswith('.d')]
        src_path = max(candidates, key=lambda p: p.stat().st_mtime)

    dest_depfile = out_path.parent / f'{out_path.name}.d'
    rewrite_depfile(src_depfile, dest_depfile, out_path)

    # Ensure parent directory of output exists
    out_path.parent.mkdir(parents=True, exist_ok=True)

    if out_path.exists() or out_path.is_symlink():
        out_path.unlink()

    rel_src = os.path.relpath(src_path, out_path.parent)
    out_path.symlink_to(rel_src)

if __name__ == '__main__':
    main()
