// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_GENERATED_FILE_WRITER_H_
#define TOOLS_GN_GENERATED_FILE_WRITER_H_

#include "gn/unique_vector.h"

class Err;
class Target;

// Write the content of a given generated_file() target to disk.
//
// If |deps_collected| is not null, the targets collected through the
// target's dep_keys during the metadata walk are added to it.
bool WriteGeneratedFileToDisk(const Target* target,
                              UniqueVector<const Target*>* deps_collected,
                              Err* err);

#endif  // TOOLS_GN_GENERATED_FILE_WRITER_H_
