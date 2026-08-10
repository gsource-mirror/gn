// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_EDIT_COMMAND_H_
#define TOOLS_GN_EDIT_COMMAND_H_

#include <memory>
#include <string>
#include <utility>
#include <vector>

#include "gn/edit/build_file_resolver.h"
#include "gn/err.h"
#include "gn/label_pattern.h"
#include "gn/value.h"

class BlockNode;
class InputFile;
class ParseNode;
class Setup;

namespace commands {

class EditCommand {
 public:
  virtual ~EditCommand() = default;

  virtual Err Apply(BuildFile& build_file) const = 0;
};

class EditTargetCommand : public EditCommand {
 public:
  Err Apply(BuildFile& build_file) const override {
    for (const auto& target : build_file.targets()) {
      Err err = Apply(build_file, target);
      if (err.has_error())
        return err;
    }
    return Ok();
  }

  // Applies this edit command to the given target in the AST.
  virtual Err Apply(BuildFile& build_file, const EditTarget& target) const = 0;
};

Err EditCommandImpl(const std::vector<std::string>& args, Setup& setup);

}  // namespace commands

#endif  // TOOLS_GN_EDIT_COMMAND_H_
