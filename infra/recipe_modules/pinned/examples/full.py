# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

DEPS = [
    'pinned',
    'recipe_engine/platform',
    'recipe_engine/step',
]


def RunSteps(api):
  packages = api.pinned.get_packages()
  api.step('packages', ['echo'] + [p.name for p in packages])


def GenTests(api):
  for platform in ('linux', 'mac', 'win'):
    yield api.test(platform) + api.platform.name(platform)
