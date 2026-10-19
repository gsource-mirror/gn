// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/check_includes_strict_exceptions.h"

#include <memory>
#include <string_view>

#include "gn/scope.h"
#include "gn/value.h"

namespace {

// Returns the path components of a directory, as a string_view.
// It is guaranteed to have exactly one trailing slash (if non-empty), and no
// leading slash
std::string_view PathComponents(const SourceDir& d) {
  return std::string_view(d.value()).substr(2);
}

// Extracts the next component from the path.
// Invariant: Path is either empty, or ends with a '/'
// Path is treated as an iterator-like object, and it is consumed.
std::string_view NextComponent(std::string_view& path) {
  if (path.empty()) {
    return "";
  }
  auto next = path.find('/');
  DCHECK(next != std::string_view::npos);
  auto out = path.substr(0, next);
  path = path.substr(next + 1);
  return out;
}

}  // namespace

void LabelPatterns::AddPattern(const LabelPattern& pattern) {
  // Should have been checked already in FromValue
  DCHECK(pattern.dir().is_source_absolute());
  std::string_view path = PathComponents(pattern.dir());
  Node* current = &root_;
  for (std::string_view comp = NextComponent(path); !comp.empty();
       comp = NextComponent(path)) {
    auto& child = current->children[std::string(comp)];
    if (!child)
      child = std::make_unique<Node>();
    current = child.get();
  }

  switch (pattern.type()) {
    case LabelPattern::RECURSIVE_DIRECTORY:
      current->is_recursive = true;
      break;
    case LabelPattern::DIRECTORY:
      current->is_directory = true;
      break;
    case LabelPattern::MATCH:
      current->targets.emplace(pattern.name());
      break;
  }
}

bool LabelPatterns::Matches(const Label& label) const {
  if (!label.dir().is_source_absolute())
    return false;
  const Node* current = &root_;
  std::string_view path = PathComponents(label.dir());
  while (true) {
    if (current->is_recursive)
      return true;
    std::string_view comp = NextComponent(path);
    if (comp.empty())
      return current->is_directory || current->targets.contains(label.name());
    auto it = current->children.find(comp);
    if (it == current->children.end())
      return false;
    current = it->second.get();
  }
}

bool CheckIncludesStrictExceptions::HasGlobalException(
    const Label& from) const {
  return global_.Matches(from);
}

bool CheckIncludesStrictExceptions::HasLocalException(const Label& from,
                                                      const Label& to) const {
  auto it = specific_targets_.find(to.GetWithNoToolchain());
  if (it != specific_targets_.end())
    return it->second.Matches(from);

  return false;
}

bool CheckIncludesStrictExceptions::HasException(const Label& from,
                                                 const Label& to) const {
  if (empty_)
    return false;
  return HasGlobalException(from) || HasLocalException(from, to);
}

Result<CheckIncludesStrictExceptions> CheckIncludesStrictExceptions::FromValue(
    const Value& value,
    const SourceDir& current_dir,
    std::string_view source_root) {
  RETURN_IF_ERROR(value.VerifyTypeIs(Value::LIST));

  CheckIncludesStrictExceptions exceptions;
  for (const auto& item : value.list_value()) {
    RETURN_IF_ERROR(item.VerifyTypeIs(Value::SCOPE));

    const Scope* scope = item.scope_value();
    const Value* includer = scope->GetValue("includer");
    if (!includer)
      return Err(item, "Exception rule requires \"includer\".");

    for (auto [key, val] : scope->GetCurrentScopeValues()) {
      if (key != "includer" && key != "included") {
        return Err(
            *val,
            "Unknown key in exception rule: \"" + std::string(key) + "\".",
            "Only \"includer\" and \"included\" are allowed.");
      }
    }

    Err err;
    LabelPattern from =
        LabelPattern::GetPattern(current_dir, source_root, *includer, &err);
    RETURN_IF_ERROR(err);

    if (!includer->string_value().starts_with("//"))
      return Err(*includer,
                 "Label pattern must be absolute (starting with '//').");

    const Value* included = scope->GetValue("included");
    if (included) {
      Label to =
          Label::Resolve(current_dir, source_root, Label(), *included, &err);
      RETURN_IF_ERROR(err);

      if (!included->string_value().starts_with("//"))
        return Err(*included,
                   "Target label must be absolute (starting with '//').");
      exceptions.AddLocalException(from, to);
    } else {
      exceptions.AddGlobalException(from);
    }
  }

  return exceptions;
}
