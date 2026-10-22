# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.
"""Pinned CIPD package versions and toolchains for GN."""

from dataclasses import dataclass
import pathlib
import shutil
import subprocess
from typing import Sequence

try:
  from .target import Arch, Target
except (ImportError, ValueError):  # pragma: no cover
  from target import Arch, Target

_CLANG_REF = 'integration'
_SYSROOT_LINUX_VERSION = 'git_revision:c912d089c3d46d8982fdef76a50514cca79b6132'
_SYSROOT_FOCAL_VERSION = 'git_revision:fa7a5a9710540f30ff98ae48b62f2cdf72ed2acd'

_HOST_CLANG = 'fuchsia/third_party/clang/${platform}'

_REPO_ROOT = pathlib.Path(__file__).parents[3]


@dataclass(frozen=True)
class Package:
  name: str
  version: str
  path: str = ''


def get_packages(host: Target, targets: Sequence[Target] = ()) -> list[Package]:
  """Returns the list of pinned CIPD packages required for host and targets."""
  packages = set()

  if host.platform.is_linux() or host.platform.is_darwin():
    packages.add(Package(_HOST_CLANG, _CLANG_REF))

  for target in targets:
    if target.platform.is_linux():
      packages.add(
          Package('fuchsia/third_party/sysroot/linux', _SYSROOT_LINUX_VERSION,
                  'sysroot'))
      # RISCV64 support starts in focal.
      if target.arch == Arch.RISCV64:
        packages.add(
            Package('fuchsia/third_party/sysroot/focal', _SYSROOT_FOCAL_VERSION,
                    'sysroot-focal'))

  return sorted(packages, key=lambda p: (p.path, p.name))


def ensure_local(packages: Sequence[Package]):  # pragma: no cover
  """Installs packages locally into root_dir via `cipd ensure`."""
  cipd_bin = shutil.which('cipd')
  if not cipd_bin:
    raise FileNotFoundError('cipd executable not found in PATH')

  lines = []
  by_subdir = {}
  for pkg in packages:
    by_subdir.setdefault(pkg.path, []).append(f'{pkg.name} {pkg.version}')

  for subdir, pkgs in by_subdir.items():
    if subdir:
      lines.append(f'@Subdir {subdir}\n')
    lines.extend([f'{p}\n' for p in pkgs])

  root_dir = _REPO_ROOT / 'clang'
  root_dir.mkdir(exist_ok=True)
  subprocess.run(
      [cipd_bin, 'ensure', '-ensure-file', '-', '-root',
       str(root_dir)],
      input=''.join(lines),
      text=True,
      check=True,
  )
  return root_dir


# Called by run_formatter.sh to ensure that the pinned clang-format is available.
if __name__ == '__main__':  # pragma: no cover
  ensure_local([Package(_HOST_CLANG, _CLANG_REF)])
