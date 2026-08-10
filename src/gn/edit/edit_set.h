// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_EDIT_EDIT_SET_H_
#define TOOLS_GN_EDIT_EDIT_SET_H_

#include <string>

#include "gn/edit_command.h"

class ParseNode;
class InputFile;

namespace commands {

class SetCommand : public EditTargetCommand {
 public:
  using EditTargetCommand::Apply;

  SetCommand(std::string attribute, std::string value_string);
  ~SetCommand() override = default;

  Err Apply(BuildFile& build_file, const EditTarget& target) const override;

 private:
  std::string attribute_;
  std::string value_string_;
};

}  // namespace commands

#endif  // TOOLS_GN_EDIT_EDIT_SET_H_
