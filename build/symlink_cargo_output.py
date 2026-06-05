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

def rewrite_depfile(src_depfiles: list[Path], dest_depfile: Path, target_name: str):
    """Orchestrates reading, parsing, merging, and writing the depfile."""
    all_deps = set()
    for src in src_depfiles:
        if src.exists():
            all_deps.update(parse_depfile(src))
    out_dir = dest_depfile.parent.resolve()
    deps_str = ' '.join(to_depfile_path(d, out_dir) for d in sorted(all_deps))
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
    if target_type == 'lib':
        src_depfiles = [cargo_out_dir / 'libgn_starlark.d']
        # Locate the static library
        src_path = cargo_out_dir / 'libgn_starlark.a'
        # Ensure parent directory of output exists
        out_path.parent.mkdir(parents=True, exist_ok=True)
        if out_path.exists() or out_path.is_symlink():
            out_path.unlink()
        rel_src = os.path.relpath(src_path, out_path.parent)
        out_path.symlink_to(rel_src)
    else:
        workspace_crates = {'runtime', 'depset', 'types', 'ffi', 'gn_starlark'}
        candidates = []
        if (cargo_out_dir / 'deps').exists():
            for c in (cargo_out_dir / 'deps').iterdir():
                if not c.name.endswith('.d') and os.access(c, os.X_OK) and c.is_file():
                    parts = c.name.split('-')
                    if len(parts) >= 2:
                        crate_name = '-'.join(parts[:-1])
                        if crate_name in workspace_crates:
                            candidates.append(c)
        groups = {}
        for c in candidates:
            parts = c.name.split('-')
            if len(parts) < 2:
                continue
            crate_name = '-'.join(parts[:-1])
            groups.setdefault(crate_name, []).append(c)

        newest_binaries = []
        for crate_name, bins in groups.items():
            newest = max(bins, key=lambda p: p.stat().st_mtime)
            newest_binaries.append(newest)

        src_depfiles = [b.parent / f'{b.name}.d' for b in newest_binaries]

        # Ensure parent directory of output exists
        out_path.parent.mkdir(parents=True, exist_ok=True)

        if out_path.exists() or out_path.is_symlink():
            out_path.unlink()

        runner_content = [
            '#!/usr/bin/env python3',
            'import subprocess',
            'import sys',
            'import os',
            '',
            'tests = ['
        ]
        for b in sorted(newest_binaries):
            rel_path = os.path.relpath(b, out_path.parent)
            runner_content.append(f'    os.path.join(os.path.dirname(__file__), {repr(rel_path)}),')
        runner_content.extend([
            ']',
            '',
            'failed = False',
            'for test in tests:',
            '    print(f"Running test binary: {os.path.basename(test)}")',
            '    res = subprocess.run([test] + sys.argv[1:])',
            '    if res.returncode != 0:',
            '        failed = True',
            'if failed:',
            '    sys.exit(1)',
        ])

        out_path.write_text('\n'.join(runner_content) + '\n')
        out_path.chmod(0o755)

    dest_depfile = out_path.parent / f'{out_path.name}.d'
    rewrite_depfile(src_depfiles, dest_depfile, out_path)

if __name__ == '__main__':
    main()
