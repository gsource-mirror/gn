// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/edit/edit_set.h"

#include <vector>

#include "gn/err.h"
#include "gn/parse_tree.h"
#include "gn/string_atom.h"

namespace commands {

SetCommand::SetCommand(std::string attribute, std::string value_string)
    : attribute_(std::move(attribute)),
      value_string_(std::move(value_string)) {}

Err SetCommand::Apply(BuildFile& build_file, const EditTarget& target) const {
  std::unique_ptr<ParseNode> value_node = build_file.parse(value_string_);

  bool replaced = false;
  for (auto& assignment : target.assignments(attribute_)) {
    if (assignment.conditional) {
      assignment.node.add_todo(
          "This is conditional, so double check this is safe to remove");
    } else if (assignment.modification) {
      assignment.node.add_todo(
          "This is a modification, so double check this is safe to remove");
    } else {
      assignment.node->AsBinaryOpMut()->set_right(
          build_file.parse(value_string_));
      replaced = true;
    }
  }

  if (!replaced) {
    // Construct new assignment: attribute = value_node.
    target.block->append_statement(build_file.create_assignment(
        attribute_, build_file.parse(value_string_)));
  }

  return Err();
}

}  // namespace commands
