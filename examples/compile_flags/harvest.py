# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

import json
from pathlib import Path
import sys

snapshot, output = map(Path, sys.argv[1:])
flags = json.loads(snapshot.read_text(encoding="utf-8"))
if flags["schema_version"] != 1:
  raise ValueError("Unsupported compile flag snapshot schema")
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(flags, sort_keys=True), encoding="utf-8")
print("Read compile flags for", flags["target"])
