# Copyright 2026 The Chromium Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

import argparse
import json
import os
from pathlib import Path
import shutil
import threading
import subprocess
import sys
import tempfile
import time


def run(*command):
  result = subprocess.run(command, capture_output=True, text=True)
  print(result.stdout, end="")
  print(result.stderr, end="", file=sys.stderr)
  result.check_returncode()
  return result.stdout


def main():
  parser = argparse.ArgumentParser()
  parser.add_argument("--gn", required=True, type=Path)
  frontend = parser.add_mutually_exclusive_group(required=True)
  frontend.add_argument("--ninja", type=Path)
  frontend.add_argument("--siso", type=Path)
  args = parser.parse_args()
  gn = str(args.gn.resolve())
  builder = ([str(args.siso.resolve()), "ninja", "-offline", "-load="]
             if args.siso else [str(args.ninja.resolve())])
  with tempfile.TemporaryDirectory(prefix="gn-compile-flags-") as tmp:
    source = Path(tmp) / "source with spaces"
    shutil.copytree(
        Path(__file__).parent,
        source,
        ignore=shutil.ignore_patterns("out", "__pycache__"),
    )
    build = source / "out"
    run(gn, "gen", str(build), "--root=" + str(source))
    run(*builder, "-C", str(build), "snapshots")
    snapshots = sorted(build.rglob("library-flags.json"))
    assert len(snapshots) == 2
    outputs = sorted(build.rglob("harvest.json"))
    assert len(outputs) == 2
    assert not list(build.rglob("*.o")), "Producer compilation was scheduled"
    toolchains = set()
    for snapshot in snapshots:
      flags = json.loads(snapshot.read_text(encoding="utf-8"))
      assert "INHERITED=1" in flags["defines"]
      assert 'MESSAGE="quoted value"' in flags["defines"]
      assert "//include with spaces/" in flags["include_dirs"]
      assert "-std=c++23" in flags["cflags_cc"]
      toolchains.add(flags["toolchain"])
    assert toolchains == {"//:host", "//:secondary"}

    mtimes = {p: p.stat().st_mtime_ns for p in snapshots + outputs}
    run(gn, "gen", str(build), "--root=" + str(source))
    assert all(p.stat().st_mtime_ns == mtimes[p] for p in snapshots)
    assert "Read compile flags" not in run(*builder, "-C", str(build),
                                           "snapshots")

    time.sleep(1.1)
    (source / "BUILD.gn").touch()
    assert "Read compile flags" not in run(*builder, "-C", str(build),
                                           "snapshots")
    assert all(p.stat().st_mtime_ns == mtimes[p] for p in snapshots + outputs)
    assert "Regenerating" not in run(*builder, "-C", str(build), "snapshots")

    time.sleep(1.1)
    build_file = source / "BUILD.gn"
    original = build_file.read_text(encoding="utf-8")
    build_file.write_text(
        original.replace("INHERITED=1", "INHERITED=2"), encoding="utf-8")
    assert run(*builder, "-C", str(build),
               "snapshots").count("Read compile flags") == 2
    for output in outputs:
      flags = json.loads(output.read_text(encoding="utf-8"))
      assert "INHERITED=2" in flags["defines"]
    assert "Regenerating" not in run(*builder, "-C", str(build), "snapshots")

    snapshots[0].unlink()
    run(*builder, "-C", str(build), "snapshots")
    assert snapshots[0].is_file(), "Deleted snapshot was not regenerated"
    assert "Regenerating" not in run(*builder, "-C", str(build), "snapshots")
    directory = snapshots[0].parent
    before = directory.stat()
    snapshots[0].unlink()
    os.utime(directory, ns=(before.st_atime_ns, before.st_mtime_ns))
    run(*builder, "-C", str(build), "snapshots")
    assert snapshots[0].is_file(
    ), "Timestamp-preserving deletion was not repaired"
    assert "Restoring" not in run(*builder, "-C", str(build), "snapshots")
    shutil.rmtree(directory)
    run(*builder, "-C", str(build), "snapshots")
    assert snapshots[0].is_file(), "Deleted directory was not restored"
    assert "Restoring" not in run(*builder, "-C", str(build), "snapshots")

    # Optional exports must not overwrite target owned snapshots.
    for switches in (
        ["--ide=json", "--json-file-name=" + str(snapshots[0])],
        ["--ninja-outputs-file=" + str(snapshots[0])],
    ):
      old = snapshots[0].read_bytes()
      result = subprocess.run(
          [gn, "gen", str(build), "--root=" + str(source)] + switches,
          capture_output=True,
          text=True,
          timeout=30)
      assert result.returncode != 0, (
          "Generation output collision was accepted", switches, result.stdout)
      assert "generation output" in result.stdout
      assert snapshots[0].read_bytes() == old

    # Invalid input must leave the last complete publication intact.
    response = build / "atomic-test.rsp"
    publication = build / "atomic-test.json"
    response.write_text('{"value": 1}', encoding="utf-8")
    run(gn, "materialize-compile-flags", str(response), str(publication))
    original_bytes = publication.read_bytes()
    response.write_text("{", encoding="utf-8")
    result = subprocess.run(
        [gn, "materialize-compile-flags",
         str(response),
         str(publication)],
        capture_output=True,
        text=True,
        timeout=30)
    assert result.returncode != 0
    assert publication.read_bytes() == original_bytes

    errors = []
    stopped = threading.Event()
    previous_publication = (publication.open("rb") if os.name != "nt" else None)

    def read_publication():
      while not stopped.is_set():
        try:
          json.loads(publication.read_text(encoding="utf-8"))
        except (OSError, ValueError) as error:
          errors.append(str(error))
          return

    reader = threading.Thread(target=read_publication)
    reader.start()
    try:
      for value in range(20):
        response.write_text(
            json.dumps({
                "value": value,
                "padding": "x" * 200000
            }),
            encoding="utf-8")
        run(gn, "materialize-compile-flags", str(response), str(publication))
    finally:
      stopped.set()
      reader.join()
    if previous_publication is not None:
      try:
        assert previous_publication.read() == original_bytes, (
            "Publication overwrote the previous file in place")
      finally:
        previous_publication.close()
    assert not errors, errors

    # Recovery reads the response data only, even if the source root is absent.
    source_marker = source / ".gn"
    source_marker.unlink()
    run(gn, "materialize-compile-flags", str(response), str(publication))
  print("Snapshot generation, toolchains, regeneration, and deletion passed.")


if __name__ == "__main__":
  main()
