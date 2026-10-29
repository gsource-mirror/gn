// Copyright 2018 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ninja_generated_file_target_writer.h"

#include <algorithm>
#include <vector>

#include "gn/deps_iterator.h"
#include "gn/generated_file_writer.h"
#include "gn/output_conversion.h"
#include "gn/output_file.h"
#include "gn/scheduler.h"
#include "gn/settings.h"
#include "gn/string_output_buffer.h"
#include "gn/string_utils.h"
#include "gn/target.h"
#include "gn/trace.h"

namespace {

// Returns false and sets |err| if |target| is a transitive dependency of one
// of |deps|, since making |target| depend on it would create a dependency
// cycle in the Ninja build plan.
bool CheckDepKeysCycle(const Target* target,
                       const UniqueVector<const Target*>& deps,
                       Err* err) {
  TargetSet visited;
  std::vector<const Target*> stack;
  for (const Target* dep : deps) {
    if (visited.add(dep))
      stack.push_back(dep);
    while (!stack.empty()) {
      const Target* cur = stack.back();
      stack.pop_back();
      if (cur == target) {
        *err = Err(target->defined_from(), "Dependency cycle via dep_keys.",
                   dep->label().GetUserVisibleName(false) +
                       " was collected through the dep_keys of " +
                       target->label().GetUserVisibleName(false) +
                       " but depends on it.");
        return false;
      }
      for (const auto& pair : cur->GetDeps(Target::DEPS_ALL)) {
        if (visited.add(pair.ptr))
          stack.push_back(pair.ptr);
      }
    }
  }
  return true;
}

}  // namespace

NinjaGeneratedFileTargetWriter::NinjaGeneratedFileTargetWriter(
    const Target* target,
    StringOutputBuffer& out)
    : NinjaTargetWriter(target, out) {}

NinjaGeneratedFileTargetWriter::~NinjaGeneratedFileTargetWriter() = default;

void NinjaGeneratedFileTargetWriter::Run() {
  // Write the file.
  UniqueVector<const Target*> deps_collected;
  Err err;
  if (!WriteGeneratedFileToDisk(target_, &err, &deps_collected) ||
      ((target_->collect_validations_metadata() ||
        !target_->walk_deps().empty()) &&
       !CheckDepKeysCycle(target_, deps_collected, &err))) {
    g_scheduler->FailWithError(err);
    return;
  }

  // A generated_file target should generate a phony target with dependencies
  // on each of the deps, data_deps, and dep_keys targets. The actual collection
  // is done at gen time, but to have correct input deps in ninja, we add output
  // from generated_file targets as deps for the stamp.
  std::vector<OutputFile> output_files = target_->computed_outputs();
  std::vector<OutputFile> data_output_files;
  const auto& target_deps = resolved().GetTargetDeps(target_);
  for (const Target* dep : target_deps.linked_deps()) {
    if (!dep->has_dependency_output()) {
      continue;
    }
    if (dep->IsDataOnly()) {
      data_output_files.push_back(dep->dependency_output());
    } else {
      output_files.push_back(dep->dependency_output());
    }
  }

  for (const Target* data_dep : target_deps.data_deps()) {
    if (data_dep->has_dependency_output())
      data_output_files.push_back(data_dep->dependency_output());
  }

  for (const Target* dep : deps_collected) {
    if (!dep->has_dependency_output())
      continue;
    auto& files = dep->IsDataOnly() ? data_output_files : output_files;
    if (std::find(files.begin(), files.end(), dep->dependency_output()) ==
        files.end())
      files.push_back(dep->dependency_output());
  }

  WriteStampOrPhonyForTarget(output_files, data_output_files);
}
