// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/compile_flags_writer.h"

#include <limits>
#include <map>

#include "base/files/file_util.h"
#include "base/json/json_writer.h"
#include "base/values.h"
#include "gn/build_settings.h"
#include "gn/desc_builder.h"
#include "gn/err.h"
#include "gn/filesystem_utils.h"
#include "gn/scheduler.h"
#include "gn/settings.h"
#include "gn/string_output_buffer.h"
#include "gn/target.h"
#include "gn/trace.h"
#include "util/atomic_write.h"

namespace {

base::FilePath CanonicalOutputPath(base::FilePath path) {
  std::vector<base::FilePath::StringType> missing;
  base::FilePath resolved = base::MakeAbsoluteFilePath(path);
  while (resolved.empty()) {
    base::FilePath parent = path.DirName();
    if (parent == path) {
      return {};
    }
    missing.push_back(path.BaseName().value());
    path = parent;
    resolved = base::MakeAbsoluteFilePath(path);
  }
  for (auto component = missing.rbegin(); component != missing.rend();
       ++component) {
    resolved = resolved.Append(*component);
  }
  return resolved;
}

}  // namespace

bool WriteCompileFlagsAtomically(const base::FilePath& path,
                                 const std::string& json,
                                 Err* err) {
  ScopedTrace trace(TraceItem::TRACE_FILE_WRITE, FilePathToUTF8(path));
  StringOutputBuffer buffer;
  buffer << json;
  if (buffer.ContentsEqual(path)) {
    return true;
  }
  if (json.size() > static_cast<size_t>(std::numeric_limits<int>::max()) ||
      !base::CreateDirectory(path.DirName()) ||
      util::WriteFileAtomically(path, json.data(),
                                static_cast<int>(json.size())) !=
          static_cast<int>(json.size())) {
    *err = Err(Location(), "Could not atomically write compile flags.");
    return false;
  }
  return true;
}

std::unique_ptr<base::DictionaryValue> ComputeCompileFlags(
    const Target* target) {
  ScopedTrace trace(
      TraceItem::TRACE_SETUP,
      "Compute compile flags " + target->label().GetUserVisibleName(true));
  trace.SetToolchain(target->settings()->toolchain_label());
  auto result = std::make_unique<base::DictionaryValue>();
  result->SetKey("schema_version", base::Value(1));
  result->SetKey("target",
                 base::Value(target->label().GetUserVisibleName(true)));
  result->SetKey(
      "toolchain",
      base::Value(
          target->settings()->toolchain_label().GetUserVisibleName(false)));
  for (const char* field :
       {"defines", "include_dirs", "framework_dirs", "asmflags", "cflags",
        "cflags_c", "cflags_cc", "cflags_objc", "cflags_objcc"}) {
    auto desc =
        DescBuilder::DescriptionForTarget(target, field, false, false, false);
    const base::Value* value = desc->FindKey(field);
    result->SetKey(
        field, value ? value->Clone() : base::Value(base::Value::Type::LIST));
  }
  return result;
}

bool WriteCompileFlagsFiles(
    const BuildSettings* build_settings,
    const std::vector<const Target*>& targets,
    Err* err,
    const std::vector<base::FilePath>& reserved_outputs) {
  std::map<OutputFile, const Target*> snapshots;
  for (const Target* target : targets) {
    const OutputFile& output = target->write_compile_flags_output();
    if (output.value().empty()) {
      continue;
    }
    auto [entry, inserted] = snapshots.emplace(output, target);
    if (!inserted) {
      *err = Err(target->defined_from(),
                 "Duplicate write_compile_flags output: " +
                     std::string(output.value()),
                 "Also requested by " +
                     entry->second->label().GetUserVisibleName(true));
      return false;
    }
  }
  if (snapshots.empty()) {
    return true;
  }
  std::map<base::FilePath, const Target*> snapshot_paths;
  if (!reserved_outputs.empty()) {
    for (const auto& [snapshot, target] : snapshots) {
      base::FilePath path = CanonicalOutputPath(
          build_settings->GetFullPath(snapshot.AsSourceFile(build_settings)));
      if (path.empty()) {
        *err = Err(target->defined_from(),
                   "Could not resolve compile flags output path.");
        return false;
      }
      snapshot_paths.emplace(path, target);
    }
  }
  for (const base::FilePath& output : reserved_outputs) {
    const base::FilePath reserved_path = CanonicalOutputPath(output);
    if (reserved_path.empty()) {
      *err = Err(Location(), "Could not resolve reserved generation output.");
      return false;
    }
    auto collision = snapshot_paths.find(reserved_path);
    if (collision != snapshot_paths.end()) {
      *err = Err(collision->second->defined_from(),
                 "write_compile_flags conflicts with a generation output: " +
                     FilePathToUTF8(output));
      return false;
    }
  }
  for (const Target* target : targets) {
    for (const OutputFile& output : target->computed_outputs()) {
      if (snapshots.contains(output)) {
        *err = Err(target->defined_from(),
                   "write_compile_flags conflicts with a target output: " +
                       std::string(output.value()));
        return false;
      }
    }
  }
  for (const auto& [output, target] : snapshots) {
    ScopedTrace trace(TraceItem::TRACE_FILE_WRITE_GENERATED,
                      output.AsSourceFile(build_settings).value());
    trace.SetToolchain(target->settings()->toolchain_label());
    auto flags = ComputeCompileFlags(target);
    std::string json;
    if (!base::JSONWriter::WriteWithOptions(
            *flags, base::JSONWriter::OPTIONS_PRETTY_PRINT, &json)) {
      *err = Err(target->defined_from(), "Could not serialize compile flags.");
      return false;
    }
    SourceFile file = output.AsSourceFile(build_settings);
    const base::FilePath path = build_settings->GetFullPath(file);
    if (!WriteCompileFlagsAtomically(path, json, err)) {
      return false;
    }
    g_scheduler->AddWrittenFile(file);
  }
  return true;
}
