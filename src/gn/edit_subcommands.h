// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_EDIT_SUBCOMMANDS_H_
#define TOOLS_GN_EDIT_SUBCOMMANDS_H_

#include <memory>
#include <string>
#include <vector>

#include "gn/err.h"

namespace commands {

class BuildFile;

class EditCommand {
 public:
  virtual ~EditCommand();

  virtual Err Apply(BuildFile& build_file) const = 0;
};

Result<std::unique_ptr<EditCommand>> ParseCommand(
    std::vector<std::string> args);

}  // namespace commands

#endif  // TOOLS_GN_EDIT_SUBCOMMANDS_H_
