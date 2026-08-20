// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/linker_inputs.h"

#include <stddef.h>

#include <algorithm>
#include <string>

#include "base/files/file_util.h"
#include "gn/scheduler.h"
#include "gn/target.h"
#include "gn/test_with_scheduler.h"
#include "gn/test_with_scope.h"
#include "util/test/test.h"

namespace {

void InitTargetWithType(TestWithScope& setup,
                        Target* target,
                        Target::OutputType type) {
  target->set_output_type(type);
  target->visibility().SetPublic();
  target->SetToolchain(setup.toolchain());
}

}  // namespace

using LinkerInputs = TestWithScheduler;

TEST_F(LinkerInputs, Basic) {
  TestWithScope setup;
  Err err;

  // Static library target.
  Target stat(setup.settings(), Label(SourceDir("//foo/"), "stat"));
  InitTargetWithType(setup, &stat, Target::STATIC_LIBRARY);
  stat.sources().push_back(SourceFile("//foo/stat.cc"));
  stat.source_types_used().Set(SourceFile::SOURCE_CPP);
  ASSERT_TRUE(stat.OnResolved(&err));

  // Shared library target.
  Target shared(setup.settings(), Label(SourceDir("//foo/"), "shared"));
  InitTargetWithType(setup, &shared, Target::SHARED_LIBRARY);
  shared.sources().push_back(SourceFile("//foo/shared.cc"));
  shared.source_types_used().Set(SourceFile::SOURCE_CPP);
  ASSERT_TRUE(shared.OnResolved(&err));

  // Source set target.
  Target set(setup.settings(), Label(SourceDir("//foo/"), "set"));
  InitTargetWithType(setup, &set, Target::SOURCE_SET);
  set.sources().push_back(SourceFile("//foo/set.cc"));
  set.source_types_used().Set(SourceFile::SOURCE_CPP);
  ASSERT_TRUE(set.OnResolved(&err));

  // Main executable target depending on stat, shared, and set.
  Target main(setup.settings(), Label(SourceDir("//foo/"), "main"));
  InitTargetWithType(setup, &main, Target::EXECUTABLE);
  main.sources().push_back(SourceFile("//foo/main.cc"));
  main.source_types_used().Set(SourceFile::SOURCE_CPP);
  main.private_deps().push_back(LabelTargetPair(&stat));
  main.private_deps().push_back(LabelTargetPair(&shared));
  main.private_deps().push_back(LabelTargetPair(&set));
  ASSERT_TRUE(main.OnResolved(&err));

  std::vector<OutputFile> inputs = ComputeLinkerInputs(&main);

  std::vector<std::string> input_strings;
  for (const OutputFile& file : inputs) {
    input_strings.emplace_back(file.value());
  }

  // Expect main's object file, set's object file, stat's archive, and shared lib.
  ASSERT_EQ(4u, input_strings.size());
  EXPECT_EQ("obj/foo/main.main.o", input_strings[0]);
  EXPECT_EQ("obj/foo/set.set.o", input_strings[1]);
  EXPECT_EQ("obj/foo/libstat.a", input_strings[2]);
  EXPECT_EQ("./libshared.so", input_strings[3]);
}

TEST_F(LinkerInputs, WriteFile) {
  TestWithScope setup;
  Err err;

  Target main(setup.settings(), Label(SourceDir("//foo/"), "main"));
  InitTargetWithType(setup, &main, Target::EXECUTABLE);
  main.sources().push_back(SourceFile("//foo/main.cc"));
  main.source_types_used().Set(SourceFile::SOURCE_CPP);
  ASSERT_TRUE(main.OnResolved(&err));

  OutputFile output_file(setup.build_settings(),
                         SourceFile("//out/Debug/linker_inputs.txt"));
  main.set_write_linker_inputs_output(output_file);

  ASSERT_TRUE(WriteLinkerInputsFile(output_file, &main, &err));
  ASSERT_FALSE(err.has_error());

  base::FilePath full_path =
      setup.build_settings()->GetFullPath(SourceFile("//out/Debug/linker_inputs.txt"));
  std::string contents;
  ASSERT_TRUE(base::ReadFileToString(full_path, &contents));
  EXPECT_EQ("obj/foo/main.main.o\n", contents);
}
