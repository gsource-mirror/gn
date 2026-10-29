// Copyright 2014 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ninja_generated_file_target_writer.h"

#include "gn/source_file.h"
#include "gn/string_output_buffer.h"
#include "gn/target.h"
#include "gn/test_with_scheduler.h"
#include "gn/test_with_scope.h"
#include "util/test/test.h"

using NinjaGeneratedFileTargetWriterTest = TestWithScheduler;

TEST_F(NinjaGeneratedFileTargetWriterTest, Run) {
  Err err;
  TestWithScope setup;

  Target target(setup.settings(), Label(SourceDir("//foo/"), "bar"));
  target.set_output_type(Target::GENERATED_FILE);
  target.visibility().SetPublic();
  target.action_values().outputs() =
      SubstitutionList::MakeForTest("//out/Debug/foo.json");
  target.set_contents(Value(nullptr, true));
  target.set_output_conversion(Value(nullptr, "json"));

  Target dep(setup.settings(), Label(SourceDir("//foo/"), "dep"));
  dep.set_output_type(Target::ACTION);
  dep.visibility().SetPublic();
  dep.SetToolchain(setup.toolchain());
  ASSERT_TRUE(dep.OnResolved(&err));

  Target dep2(setup.settings(), Label(SourceDir("//foo/"), "dep2"));
  dep2.set_output_type(Target::ACTION);
  dep2.visibility().SetPublic();
  dep2.SetToolchain(setup.toolchain());
  ASSERT_TRUE(dep2.OnResolved(&err));

  Target bundle_data_dep(setup.settings(),
                         Label(SourceDir("//foo/"), "bundle_data_dep"));
  bundle_data_dep.sources().push_back(SourceFile("//foo/some_data.txt"));
  bundle_data_dep.set_output_type(Target::BUNDLE_DATA);
  bundle_data_dep.visibility().SetPublic();
  bundle_data_dep.SetToolchain(setup.toolchain());
  ASSERT_TRUE(bundle_data_dep.OnResolved(&err));

  Target datadep(setup.settings(), Label(SourceDir("//foo/"), "datadep"));
  datadep.set_output_type(Target::ACTION);
  datadep.visibility().SetPublic();
  datadep.SetToolchain(setup.toolchain());
  ASSERT_TRUE(datadep.OnResolved(&err));

  target.public_deps().push_back(LabelTargetPair(&dep));
  target.public_deps().push_back(LabelTargetPair(&dep2));
  target.public_deps().push_back(LabelTargetPair(&bundle_data_dep));
  target.data_deps().push_back(LabelTargetPair(&datadep));

  target.SetToolchain(setup.toolchain());
  ASSERT_TRUE(target.OnResolved(&err)) << err.message();

  StringOutputBuffer out;
  bool delayed_write = false;
  NinjaGeneratedFileTargetWriter writer(&target, out);
  writer.SetDelayedGeneratedFileWritePtr(&delayed_write);
  writer.Run();

  const char expected[] =
      "build phony/foo/bar: phony foo.json phony/foo/dep "
      "phony/foo/dep2 || "
      "phony/foo/bundle_data_dep phony/foo/datadep\n";
  EXPECT_EQ(expected, out);
  EXPECT_FALSE(delayed_write);
}

TEST_F(NinjaGeneratedFileTargetWriterTest, RunWithDelayedGeneratedFileWrite) {
  Err err;
  TestWithScope setup;

  Target target(setup.settings(), Label(SourceDir("//foo/"), "bar"));
  target.set_output_type(Target::GENERATED_FILE);
  target.visibility().SetPublic();
  target.action_values().outputs() =
      SubstitutionList::MakeForTest("//out/Debug/foo.json");
  target.set_contents(Value(nullptr, true));
  target.set_output_conversion(Value(nullptr, "json"));
  target.set_collect_validations_metadata(true);

  Target dep(setup.settings(), Label(SourceDir("//foo/"), "dep"));
  dep.set_output_type(Target::ACTION);
  dep.visibility().SetPublic();
  dep.SetToolchain(setup.toolchain());
  ASSERT_TRUE(dep.OnResolved(&err));

  Target dep2(setup.settings(), Label(SourceDir("//foo/"), "dep2"));
  dep2.set_output_type(Target::ACTION);
  dep2.visibility().SetPublic();
  dep2.SetToolchain(setup.toolchain());
  ASSERT_TRUE(dep2.OnResolved(&err));

  Target bundle_data_dep(setup.settings(),
                         Label(SourceDir("//foo/"), "bundle_data_dep"));
  bundle_data_dep.sources().push_back(SourceFile("//foo/some_data.txt"));
  bundle_data_dep.set_output_type(Target::BUNDLE_DATA);
  bundle_data_dep.visibility().SetPublic();
  bundle_data_dep.SetToolchain(setup.toolchain());
  ASSERT_TRUE(bundle_data_dep.OnResolved(&err));

  Target datadep(setup.settings(), Label(SourceDir("//foo/"), "datadep"));
  datadep.set_output_type(Target::ACTION);
  datadep.visibility().SetPublic();
  datadep.SetToolchain(setup.toolchain());
  ASSERT_TRUE(datadep.OnResolved(&err));

  target.public_deps().push_back(LabelTargetPair(&dep));
  target.public_deps().push_back(LabelTargetPair(&dep2));
  target.public_deps().push_back(LabelTargetPair(&bundle_data_dep));
  target.data_deps().push_back(LabelTargetPair(&datadep));

  target.SetToolchain(setup.toolchain());
  ASSERT_TRUE(target.OnResolved(&err)) << err.message();

  StringOutputBuffer out;
  NinjaGeneratedFileTargetWriter writer(&target, out);
  bool delayed_write = false;
  writer.SetDelayedGeneratedFileWritePtr(&delayed_write);
  writer.Run();

  // Nothing is written when delayed.
  EXPECT_EQ("", out.str());
  EXPECT_TRUE(delayed_write);

  // Running again without a delay pointer writes the rule.
  NinjaGeneratedFileTargetWriter writer2(&target, out);
  writer2.Run();

  const char expected[] =
      "build phony/foo/bar: phony foo.json phony/foo/dep "
      "phony/foo/dep2 || "
      "phony/foo/bundle_data_dep phony/foo/datadep\n";
  EXPECT_EQ(expected, out);
}

// Tests that targets collected through dep_keys are added to the rule, and
// that targets listed in walk_deps are not.
TEST_F(NinjaGeneratedFileTargetWriterTest, RunWithWalkDepsAndDepKeys) {
  Err err;
  TestWithScope setup;

  Target pickle(setup.settings(), Label(SourceDir("//foo/"), "pickle"));
  pickle.set_output_type(Target::ACTION);
  pickle.visibility().SetPublic();
  pickle.SetToolchain(setup.toolchain());
  Value pickle_files(nullptr, Value::LIST);
  pickle_files.list_value().push_back(Value(nullptr, "foo.pickle"));
  pickle.metadata().contents().emplace("files", pickle_files);
  Value pickle_deps(nullptr, Value::LIST);
  pickle_deps.list_value().push_back(Value(nullptr, ":pickle"));
  pickle.metadata().contents().emplace("file_deps", pickle_deps);
  ASSERT_TRUE(pickle.OnResolved(&err));

  Target lib(setup.settings(), Label(SourceDir("//foo/"), "lib"));
  lib.set_output_type(Target::ACTION);
  lib.visibility().SetPublic();
  lib.SetToolchain(setup.toolchain());
  lib.private_deps().push_back(LabelTargetPair(&pickle));
  ASSERT_TRUE(lib.OnResolved(&err));

  Target target(setup.settings(), Label(SourceDir("//foo/"), "bar"));
  target.set_output_type(Target::GENERATED_FILE);
  target.visibility().SetPublic();
  target.action_values().outputs() =
      SubstitutionList::MakeForTest("//out/Debug/foo.json");
  target.set_output_conversion(Value(nullptr, "json"));
  target.data_keys().push_back("files");
  target.walk_keys().push_back("");
  target.dep_keys().push_back("file_deps");
  target.walk_deps().push_back(LabelTargetPair(&lib));
  // Also a direct dependency, which should not be listed twice.
  target.private_deps().push_back(LabelTargetPair(&pickle));
  target.SetToolchain(setup.toolchain());
  ASSERT_TRUE(target.OnResolved(&err)) << err.message();

  // The metadata walk is delayed because of walk_deps.
  StringOutputBuffer out;
  bool delayed_write = false;
  NinjaGeneratedFileTargetWriter writer(&target, out);
  writer.SetDelayedGeneratedFileWritePtr(&delayed_write);
  writer.Run();
  EXPECT_EQ("", out.str());
  EXPECT_TRUE(delayed_write);

  NinjaGeneratedFileTargetWriter writer2(&target, out);
  writer2.Run();
  EXPECT_FALSE(scheduler().is_failed());

  const char expected[] =
      "build phony/foo/bar: phony foo.json phony/foo/pickle\n";
  EXPECT_EQ(expected, out);
}

// Tests that an error is reported when a target collected through dep_keys
// depends on the generated_file() target.
TEST_F(NinjaGeneratedFileTargetWriterTest, RunWithDepKeysCycle) {
  Err err;
  TestWithScope setup;

  Target target(setup.settings(), Label(SourceDir("//foo/"), "bar"));
  target.set_output_type(Target::GENERATED_FILE);
  target.visibility().SetPublic();
  target.action_values().outputs() =
      SubstitutionList::MakeForTest("//out/Debug/foo.json");
  target.set_output_conversion(Value(nullptr, "json"));
  target.data_keys().push_back("files");
  target.walk_keys().push_back("");
  target.dep_keys().push_back("file_deps");
  target.SetToolchain(setup.toolchain());
  ASSERT_TRUE(target.OnResolved(&err)) << err.message();

  // Depends on the generated_file() target and lists itself under dep_keys.
  Target action(setup.settings(), Label(SourceDir("//foo/"), "action"));
  action.set_output_type(Target::ACTION);
  action.visibility().SetPublic();
  action.SetToolchain(setup.toolchain());
  action.private_deps().push_back(LabelTargetPair(&target));
  Value action_deps(nullptr, Value::LIST);
  action_deps.list_value().push_back(Value(nullptr, ":action"));
  action.metadata().contents().emplace("file_deps", action_deps);
  ASSERT_TRUE(action.OnResolved(&err));

  target.walk_deps().push_back(LabelTargetPair(&action));

  StringOutputBuffer out;
  NinjaGeneratedFileTargetWriter writer(&target, out);
  writer.Run();
  EXPECT_TRUE(scheduler().is_failed());
}
