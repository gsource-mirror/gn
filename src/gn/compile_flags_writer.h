// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_COMPILE_FLAGS_WRITER_H_
#define TOOLS_GN_COMPILE_FLAGS_WRITER_H_

#include <memory>
#include <string>
#include <vector>

#include "base/files/file_path.h"

namespace base {
class DictionaryValue;
}  // namespace base

class BuildSettings;
class Err;
class Target;

bool WriteCompileFlagsAtomically(const base::FilePath& path,
                                 const std::string& json,
                                 Err* err);

std::unique_ptr<base::DictionaryValue> ComputeCompileFlags(
    const Target* target);

bool WriteCompileFlagsFiles(
    const BuildSettings* build_settings,
    const std::vector<const Target*>& targets,
    Err* err,
    const std::vector<base::FilePath>& reserved_outputs = {});

#endif  // TOOLS_GN_COMPILE_FLAGS_WRITER_H_
