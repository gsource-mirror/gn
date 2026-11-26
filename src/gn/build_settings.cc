// Copyright (c) 2013 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/build_settings.h"

#include <utility>

#include "base/files/file_util.h"
#include "cxx.h"
#include "gn/ffi/bridge.h"
#include "gn/filesystem_utils.h"

namespace {

// Computes the prefix for any source-relative OutputFile.
// Eg. if the build directory is //out/Default, it would return ../../
std::string ComputeSourceRootRel(const SourceDir& build_dir,
                                 std::string_view root_path_utf8) {
  std::string source_root_rel = RebasePath("//", build_dir, root_path_utf8);
  if (!EndsWithSlash(source_root_rel)) {
    source_root_rel.push_back('/');
  }
  return source_root_rel;
}

void DropSession(std::atomic<Session*>& slot) {
  if (Session* session = slot.exchange(nullptr, std::memory_order_acq_rel)) {
    rust::Box<Session>::from_raw(session);
  }
}

}  // namespace

BuildSettings::BuildSettings() = default;

BuildSettings::BuildSettings(const BuildSettings& other)
    : dotfile_name_(other.dotfile_name_),
      root_path_(other.root_path_),
      root_path_utf8_(other.root_path_utf8_),
      secondary_source_path_(other.secondary_source_path_),
      python_path_(other.python_path_),
      python_path_is_relative_to_build_dir_(
          other.python_path_is_relative_to_build_dir_),
      ninja_required_version_(other.ninja_required_version_),
      build_config_file_(other.build_config_file_),
      arg_file_template_path_(other.arg_file_template_path_),
      build_dir_(other.build_dir_),
      build_args_(other.build_args_) {}

BuildSettings::~BuildSettings() {
  DropSession(starlark_session_);
}

void BuildSettings::SetRootTargetLabel(const Label& r) {
  root_target_label_ = r;
}

void BuildSettings::SetRootPatterns(std::vector<LabelPattern>&& patterns) {
  root_patterns_ = std::move(patterns);
}

void BuildSettings::SetRootPath(const base::FilePath& r) {
  DCHECK(r.value()[r.value().size() - 1] != base::FilePath::kSeparators[0]);
  root_path_ = r.NormalizePathSeparatorsTo('/');
  root_path_utf8_ = FilePathToUTF8(root_path_);
  DropSession(starlark_session_);
}

void BuildSettings::SetSecondarySourcePath(const SourceDir& d) {
  secondary_source_path_ = GetFullPath(d).NormalizePathSeparatorsTo('/');
}

void BuildSettings::SetPythonPath(base::FilePath p) {
  python_path_ = std::move(p);
  python_path_is_relative_to_build_dir_ = false;

  // If the provided python path is absolute and is within the root source tree,
  // make it relative to the build directory. This helps keep generated ninja
  // files free of absolute paths when possible.
  if (python_path_.IsAbsolute() && root_path_.IsParent(python_path_)) {
    python_path_ = UTF8ToFilePath(
        RebasePath(FilePathToUTF8(python_path_), build_dir_, root_path_utf8_));
    python_path_is_relative_to_build_dir_ = true;
  }
}

void BuildSettings::SetBuildDir(const SourceDir& d) {
  build_dir_ = d;
  DropSession(starlark_session_);
}

base::FilePath BuildSettings::GetFullPath(const SourceFile& file) const {
  return file.Resolve(root_path_).NormalizePathSeparatorsTo('/');
}

base::FilePath BuildSettings::GetFullPath(const SourceDir& dir) const {
  return dir.Resolve(root_path_).NormalizePathSeparatorsTo('/');
}

base::FilePath BuildSettings::GetFullPath(const std::string& path,
                                          bool as_file) const {
  return ResolvePath(path, as_file, root_path_).NormalizePathSeparatorsTo('/');
}

base::FilePath BuildSettings::GetFullPathSecondary(
    const SourceFile& file) const {
  return file.Resolve(secondary_source_path_).NormalizePathSeparatorsTo('/');
}

base::FilePath BuildSettings::GetFullPathSecondary(const SourceDir& dir) const {
  return dir.Resolve(secondary_source_path_).NormalizePathSeparatorsTo('/');
}

base::FilePath BuildSettings::GetFullPathSecondary(const std::string& path,
                                                   bool as_file) const {
  return ResolvePath(path, as_file, secondary_source_path_)
      .NormalizePathSeparatorsTo('/');
}

void BuildSettings::ItemDefined(std::unique_ptr<Item> item) const {
  DCHECK(item);
  if (item_defined_callback_)
    item_defined_callback_(std::move(item));
}

const BuildSettings::PrintCallback BuildSettings::swap_print_callback(
    const BuildSettings::PrintCallback callback) {
  auto temp = std::move(print_callback_);
  print_callback_ = callback;
  return temp;
}

const Session& BuildSettings::starlark_session() const {
  Session* session = starlark_session_.load(std::memory_order_acquire);
  if (UNLIKELY(!session)) {
    auto new_session =
        root_path_.empty() || build_dir_.is_null()
            ? Session::new_for_testing()
            : Session::new_cxx(
                  root_path_utf8_, FilePathToUTF8(GetFullPath(build_dir_)),
                  ComputeSourceRootRel(build_dir_, root_path_utf8_));
    Session* raw = new_session.into_raw();
    if (starlark_session_.compare_exchange_strong(
            session, raw, std::memory_order_acq_rel,
            std::memory_order_acquire)) {
      session = raw;
    } else {
      rust::Box<Session>::from_raw(raw);
    }
  }
  return *session;
}