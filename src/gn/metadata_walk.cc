// Copyright 2018 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/metadata_walk.h"

std::vector<Value> WalkMetadata(
    const UniqueVector<const Target*>& targets_to_walk,
    const Metadata::KeyList& data_keys,
    const Metadata::KeyList& walk_keys,
    const SourceDir& rebase_dir,
    TargetSet* targets_walked,
    Err* err) {
  std::vector<Value> result;
  for (const auto* target : targets_to_walk) {
    if (targets_walked->add(target)) {
      if (!target->GetMetadata(data_keys, walk_keys, rebase_dir, false, &result,
                               targets_walked, err))
        return std::vector<Value>();
    }
  }
  return result;
}
