// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/edit_subcommands.h"

#include "gn/edit/build_file_resolver.h"
#include "gn/err.h"
#include "gn/location.h"
#include "gn/parse_tree.h"

namespace commands {

EditCommand::~EditCommand() = default;

class EditTargetCommand : public EditCommand {
 public:
  Err Apply(BuildFile& build_file) const override {
    for (const auto& target : build_file.targets()) {
      Err err = ApplyToTarget(build_file, target);
      if (err.has_error())
        return err;
    }
    return Ok();
  }

  // Applies this edit command to the given target in the AST.
  virtual Err ApplyToTarget(BuildFile& build_file, const EditTarget& target) const = 0;
};

class SetCommand : public EditTargetCommand {
 public:
  SetCommand(std::string attribute, std::string value_string)
      : attribute_(std::move(attribute)),
        value_string_(std::move(value_string)) {}

  Err ApplyToTarget(BuildFile& build_file, const EditTarget& target) const override {
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

    return Ok();
  }

 private:
  std::string attribute_;
  std::string value_string_;
};

Result<std::unique_ptr<EditCommand>> ParseCommand(
    std::vector<std::string> args) {
  if (args.empty()) {
    return Err(Location(), "Empty command.");
  }

  if (args[0] == "set") {
    if (args.size() != 3) {
      return Err(Location(), "Invalid set command.",
                 "Usage: set <attribute> <value>");
    }
    return std::unique_ptr<EditCommand>(
        std::make_unique<SetCommand>(args[1], args[2]));
  }

  return Err(Location(), "Unknown command action: " + args[0],
             "Currently only \"set\" is supported.");
}

}  // namespace commands
