// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/linker_inputs.h"

#include <ostream>

#include "gn/build_settings.h"
#include "gn/builder.h"
#include "gn/err.h"
#include "gn/filesystem_utils.h"
#include "gn/ninja_binary_target_writer.h"
#include "gn/output_file.h"
#include "gn/resolved_target_data.h"
#include "gn/scheduler.h"
#include "gn/string_output_buffer.h"
#include "gn/target.h"
#include "gn/trace.h"
#include "gn/unique_vector.h"

std::vector<OutputFile> ComputeLinkerInputs(const Target* target) {
  UniqueVector<OutputFile> inputs;
  if (!target->IsFinal())
    return inputs.vector();

  ResolvedTargetData resolved;

  // 1. Target's own object files (and resources, MSVC PCH objects, etc.).
  NinjaBinaryTargetWriter::AddSourceSetFiles(target, &inputs);

  NinjaBinaryTargetWriter::ClassifiedDeps classified =
      NinjaBinaryTargetWriter::GetClassifiedDeps(target, resolved);

  // 2. Extra object files from source sets and incomplete static libraries.
  inputs.Append(classified.extra_object_files.begin(),
                classified.extra_object_files.end());

  // 3. Linkable target dependencies (static libraries, import libraries, etc.).
  for (const Target* dep : classified.linkable_deps) {
    if (dep->output_type() == Target::RUST_LIBRARY ||
        dep->output_type() == Target::RUST_PROC_MACRO) {
      continue;
    }
    if (!dep->link_output_file().value().empty()) {
      inputs.push_back(dep->link_output_file());
    }
  }

  // 4. Libraries specified by paths in "libs".
  for (const auto& lib : resolved.GetLinkedLibraries(target)) {
    if (lib.is_source_file()) {
      inputs.push_back(
          OutputFile(target->settings()->build_settings(), lib.source_file()));
    }
  }

  // 5. Transitive Rust libraries (.rlib).
  for (const auto& inherited : resolved.GetInheritedLibraries(target)) {
    const Target* dep = inherited.target();
    if (dep->output_type() == Target::RUST_LIBRARY &&
        dep->has_dependency_output_file()) {
      inputs.push_back(dep->dependency_output_file());
    }
  }

  return inputs.vector();
}

bool WriteLinkerInputsFile(const OutputFile& output_file,
                           const Target* target,
                           Err* err) {
  SourceFile output_as_source =
      output_file.AsSourceFile(target->settings()->build_settings());
  base::FilePath linker_inputs_file =
      target->settings()->build_settings()->GetFullPath(output_as_source);

  StringOutputBuffer storage;
  std::ostream contents(&storage);
  for (const OutputFile& input : ComputeLinkerInputs(target)) {
    contents << input.value() << std::endl;
  }

  ScopedTrace trace(TraceItem::TRACE_FILE_WRITE, output_as_source.value());
  return storage.WriteToFileIfChanged(linker_inputs_file, err);
}

bool WriteLinkerInputsFilesIfNecessary(const BuildSettings* build_settings,
                                       const Builder& builder) {
  for (const Target* target : g_scheduler->GetWriteLinkerInputsTargets()) {
    g_scheduler->ScheduleWork(
        [output_file = target->write_linker_inputs_output(), target]() {
          Err err;
          if (!WriteLinkerInputsFile(output_file, target, &err)) {
            g_scheduler->FailWithError(err);
          }
        });
  }

  return g_scheduler->Run();
}
