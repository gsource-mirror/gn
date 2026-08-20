// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_LINKER_INPUTS_H_
#define TOOLS_GN_LINKER_INPUTS_H_

#include <utility>
#include <vector>

class Builder;
class BuildSettings;
class Err;
class OutputFile;
class Target;

using LinkerInputsVector = std::vector<std::pair<OutputFile, const Target*>>;

// Computes the linker inputs (object files, static libraries, import libraries,
// etc.) for the given target.
std::vector<OutputFile> ComputeLinkerInputs(const Target* target);

// Writes all linker inputs files that were requested by targets with the
// write_linker_inputs property.
bool WriteLinkerInputsFilesIfNecessary(const BuildSettings* build_settings,
                                       const Builder& builder);

// Writes a specific linker inputs file for the given target.
bool WriteLinkerInputsFile(const OutputFile& output_file,
                           const Target* target,
                           Err* err);

#endif  // TOOLS_GN_LINKER_INPUTS_H_
