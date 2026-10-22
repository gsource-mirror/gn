# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

from recipe_engine import recipe_api

from . import pinned
from . import target


def _to_target(t):
  return target.Target(target.Platform(t.os), target.Arch(t.arch))


class PinnedApi(recipe_api.RecipeApi):
  Package = pinned.Package
  Target = target.Target

  def release_targets(self):
    if self.m.platform.is_linux:
      return [
          self.m.target('linux-amd64'),
          self.m.target('linux-arm64'),
          self.m.target('linux-riscv64')
      ]
    elif self.m.platform.is_mac:
      return [self.m.target('mac-amd64'), self.m.target('mac-arm64')]
    else:
      return [self.m.target.host]

  def get_packages(self):
    return pinned.get_packages(
        _to_target(self.m.target.host),
        [_to_target(t) for t in self.release_targets()],
    )
