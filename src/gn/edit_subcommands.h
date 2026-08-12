// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_EDIT_SUBCOMMANDS_H_
#define TOOLS_GN_EDIT_SUBCOMMANDS_H_

#include <memory>
#include <set>
#include <string>
#include <vector>

#include "gn/err.h"
#include "gn/label.h"

namespace commands {

class BuildFile;

struct EditState {
  // Each label in this list needs manual review. # TODO(gn edit) comments
  // will have been added to the build file to give the user more precise
  // instructions.
  std::set<Label> needs_manual_review;
  // Extra information the user should be wary of. For example, if the user
  // runs: `gn edit "remove deps //bar" //foo`, but //bar was not a dependency
  // of //foo.
  std::vector<Err> warnings;
};

class EditCommand {
 public:
  virtual ~EditCommand();

  virtual Err Apply(BuildFile& build_file, EditState& state) const = 0;
};

Result<std::unique_ptr<EditCommand>> ParseCommand(
    std::vector<std::string> args);

}  // namespace commands

#endif  // TOOLS_GN_EDIT_SUBCOMMANDS_H_
