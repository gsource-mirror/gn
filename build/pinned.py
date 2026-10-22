# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.
"""Pinned CIPD package versions and toolchains for GN."""

import argparse
from dataclasses import dataclass
import pathlib
import shutil
import subprocess

from target import Arch, Platform, Target

# https://chrome-infra-packages.appspot.com/p/fuchsia/third_party/clang
# https://llvm.googlesource.com/llvm-project
_CLANG_VERSION = 'integration'

# https://chrome-infra-packages.appspot.com/p/fuchsia/third_party/sysroot/linux
_SYSROOT_LINUX_VERSION = 'git_revision:c912d089c3d46d8982fdef76a50514cca79b6132'

# https://chrome-infra-packages.appspot.com/p/fuchsia/third_party/sysroot/focal
_SYSROOT_FOCAL_VERSION = 'git_revision:fa7a5a9710540f30ff98ae48b62f2cdf72ed2acd'

_HOST_CLANG = 'fuchsia/third_party/clang/${platform}'
_NINJA = 'infra/ninja/${platform}'
_NINJA_VERSION = 'version:1.8.2'

_REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]
_DEFAULT_INSTALL_DIR = _REPO_ROOT / 'clang'


@dataclass(frozen=True)
class Package:
  name: str
  version: str
  path: str = ''


def get_packages(host: Target,
                 targets: list[Target] = (),
                 ci: bool = False) -> list[Package]:
  """Returns the list of pinned CIPD packages required for host and targets."""
  packages = set()

  if ci:
    packages.add(Package(_NINJA, _NINJA_VERSION))

  if host.platform.is_linux() or host.platform.is_darwin():
    packages.add(Package(_HOST_CLANG, _CLANG_VERSION))

  for target in targets:
    if target.platform.is_linux():
      # RISCV64 support starts in focal.
      if target.arch == Arch.RISCV64:
        # In CI, we need to have multiple sysroots available, so we need a distinct name.
        name = 'sysroot-focal' if ci else 'sysroot'
        packages.add(
            Package('fuchsia/third_party/sysroot/focal', _SYSROOT_FOCAL_VERSION,
                    name))
      else:
        packages.add(
            Package('fuchsia/third_party/sysroot/linux', _SYSROOT_LINUX_VERSION,
                    'sysroot'))

  return sorted(packages, key=lambda p: (p.path, p.name))


def install(packages: list[Package],
                 install_dir: pathlib.Path = _DEFAULT_INSTALL_DIR):
  """Installs packages locally into root_dir via `cipd ensure`."""
  cipd_bin = shutil.which('cipd')
  if not cipd_bin:
    raise FileNotFoundError('cipd executable not found in PATH')

  lines = []
  by_subdir = {}
  for pkg in packages:
    by_subdir.setdefault(pkg.path, []).append(f'{pkg.name} {pkg.version}')

  for subdir, pkgs in by_subdir.items():
    lines.append(f'@Subdir {subdir}\n' if subdir else '@Subdir\n')
    lines.extend([f'{p}\n' for p in pkgs])

  install_dir.mkdir(exist_ok=True)
  subprocess.run(
      [cipd_bin, 'ensure', '-ensure-file', '-', '-root',
       str(install_dir)],
      input=''.join(lines),
      text=True,
      check=True,
  )
  return install_dir

def _parse_target(target: str):
  os, cpu = target.split('-', 1)
  return Target(Platform(os), Arch(cpu))

def main():
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument(
      '--host',
      type=_parse_target,
      default=Target.host(),
      help='Host platform (e.g. linux-amd64)',
  )
  parser.add_argument(
      '--target',
      nargs='*',
      type=_parse_target,
      default=[],
      help='Target platforms',
  )
  parser.add_argument(
      '--ci',
      type=pathlib.Path,
      help='Directory to install packages into for CI',
  )
  args = parser.parse_args()

  packages = get_packages(host=args.host, targets=args.target, ci=bool(args.ci))
  install(packages, install_dir=args.ci or _DEFAULT_INSTALL_DIR)


if __name__ == '__main__':
  main()
