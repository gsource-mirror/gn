// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/compile_flags_writer.h"

#include "base/values.h"
#include "gn/config.h"
#include "gn/target.h"
#include "gn/test_with_scheduler.h"
#include "gn/test_with_scope.h"
#include "util/test/test.h"

using CompileFlagsWriterTest = TestWithScheduler;

TEST_F(CompileFlagsWriterTest, ResolvedConfigsAndOrdering) {
  TestWithScope setup;
  Err err;
  Config nested(setup.settings(), setup.ParseLabel("//:nested"));
  nested.visibility().SetPublic();
  nested.own_values().defines() = {"COMMON", "NESTED"};
  nested.own_values().cflags() = {"-duplicate"};
  ASSERT_TRUE(nested.OnResolved(&err));

  Config local(setup.settings(), setup.ParseLabel("//:local"));
  local.visibility().SetPublic();
  local.own_values().defines() = {"LOCAL"};
  local.configs().push_back(LabelConfigPair(&nested));
  ASSERT_TRUE(local.OnResolved(&err));

  Config public_config(setup.settings(), setup.ParseLabel("//:public"));
  public_config.visibility().SetPublic();
  public_config.own_values().defines() = {"PUBLIC"};
  public_config.own_values().include_dirs() = {SourceDir("//dep/include/")};
  public_config.own_values().cflags_cc() = {"-std=c++23"};
  ASSERT_TRUE(public_config.OnResolved(&err));

  Target dep(setup.settings(), setup.ParseLabel("//:dep"));
  dep.visibility().SetPublic();
  dep.set_output_type(Target::SOURCE_SET);
  dep.SetToolchain(setup.toolchain());
  dep.public_configs().push_back(LabelConfigPair(&public_config));
  ASSERT_TRUE(dep.OnResolved(&err)) << err.message();

  Target target(setup.settings(), setup.ParseLabel("//:library"));
  target.set_output_type(Target::SOURCE_SET);
  target.SetToolchain(setup.toolchain());
  target.config_values().defines() = {"TARGET", "COMMON"};
  target.config_values().cflags() = {"-duplicate"};
  target.configs().push_back(LabelConfigPair(&local));
  target.private_deps().push_back(LabelTargetPair(&dep));
  ASSERT_TRUE(target.OnResolved(&err)) << err.message();

  auto flags = ComputeCompileFlags(&target);
  EXPECT_EQ(1, flags->FindKey("schema_version")->GetInt());
  EXPECT_EQ("//:library(//toolchain:default)",
            flags->FindKey("target")->GetString());
  EXPECT_EQ("//toolchain:default", flags->FindKey("toolchain")->GetString());
  const auto& defines = flags->FindKey("defines")->GetList();
  ASSERT_EQ(5u, defines.size());
  EXPECT_EQ("TARGET", defines[0].GetString());
  EXPECT_EQ("COMMON", defines[1].GetString());
  EXPECT_EQ("LOCAL", defines[2].GetString());
  EXPECT_EQ("NESTED", defines[3].GetString());
  EXPECT_EQ("PUBLIC", defines[4].GetString());
  const auto& cflags = flags->FindKey("cflags")->GetList();
  ASSERT_EQ(2u, cflags.size());
  EXPECT_EQ("-duplicate", cflags[0].GetString());
  EXPECT_EQ("-duplicate", cflags[1].GetString());
  EXPECT_EQ("//dep/include/",
            flags->FindKey("include_dirs")->GetList()[0].GetString());
  EXPECT_EQ("-std=c++23",
            flags->FindKey("cflags_cc")->GetList()[0].GetString());
  EXPECT_TRUE(flags->FindKey("cflags_c")->GetList().empty());
}

TEST_F(CompileFlagsWriterTest, ExportPathValidation) {
  for (const char* value :
       {"17", "\"\"", "\"//flags.json\"", "\"//out/Debug/build.ninja\"",
        "\"//out/Debug/obj/library.ninja\"", "\"//out/Debug/args.gn\""}) {
    TestWithScope setup;
    Err err;
    EXPECT_FALSE(setup.ExecuteSnippet(
        std::string("source_set(\"library\") { write_compile_flags = ") +
            value + " }",
        &err));
    EXPECT_TRUE(err.has_error());
  }
  TestWithScope setup;
  Err err;
  ASSERT_TRUE(setup.ExecuteSnippet(
      "source_set(\"library\") {"
      "write_compile_flags = \"//out/Debug/gen/flags.json\" }",
      &err));
  const Target* target = setup.items().back()->AsTarget();
  EXPECT_EQ("gen/flags.json", target->write_compile_flags_output().value());
}

TEST_F(CompileFlagsWriterTest, DuplicateSnapshotsRejected) {
  TestWithScope setup;
  Target first(setup.settings(), setup.ParseLabel("//:first"));
  Target second(setup.settings(), setup.ParseLabel("//:second"));
  first.set_write_compile_flags_output(OutputFile("flags.json"));
  second.set_write_compile_flags_output(OutputFile("flags.json"));
  Err err;
  EXPECT_FALSE(
      WriteCompileFlagsFiles(setup.build_settings(), {&first, &second}, &err));
  EXPECT_NE(std::string::npos, err.message().find("Duplicate"));
}

TEST_F(CompileFlagsWriterTest, TargetOutputCollisionRejected) {
  TestWithScope setup;
  TestTarget producer(setup, "//:producer", Target::SOURCE_SET);
  producer.set_write_compile_flags_output(OutputFile("flags.json"));
  TestTarget action(setup, "//:action", Target::ACTION);
  action.action_values().set_script(SourceFile("//script.py"));
  action.action_values().outputs() =
      SubstitutionList::MakeForTest("//out/Debug/flags.json");
  Err err;
  ASSERT_TRUE(action.OnResolved(&err));
  EXPECT_FALSE(WriteCompileFlagsFiles(setup.build_settings(),
                                      {&producer, &action}, &err));
  EXPECT_NE(std::string::npos, err.message().find("conflicts"));
}

TEST_F(CompileFlagsWriterTest, GenerationOutputCollisionRejected) {
  TestWithScope setup;
  Target target(setup.settings(), setup.ParseLabel("//:library"));
  target.set_write_compile_flags_output(OutputFile("project.json"));
  Err err;
  EXPECT_FALSE(
      WriteCompileFlagsFiles(setup.build_settings(), {&target}, &err,
                             {setup.build_settings()->GetFullPath(
                                 SourceFile("//out/Debug/project.json"))}));
  EXPECT_NE(std::string::npos, err.message().find("generation output"));
}
