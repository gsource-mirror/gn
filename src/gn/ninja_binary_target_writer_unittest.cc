// Copyright (c) 2013 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/ninja_binary_target_writer.h"

#include "base/files/scoped_temp_dir.h"
#include "gn/config.h"
#include "gn/string_output_buffer.h"
#include "gn/test_with_scheduler.h"
#include "gn/test_with_scope.h"
#include "util/test/test.h"

using NinjaBinaryTargetWriterTest = TestWithScheduler;

TEST_F(NinjaBinaryTargetWriterTest, CSources) {
  Err err;
  TestWithScope setup;

  Target target(setup.settings(), Label(SourceDir("//foo/"), "bar"));
  target.set_output_type(Target::SOURCE_SET);
  target.visibility().SetPublic();
  target.sources().push_back(SourceFile("//foo/input1.cc"));
  target.sources().push_back(SourceFile("//foo/input2.cc"));
  // Also test object files, which should be just passed through to the
  // dependents to link.
  target.sources().push_back(SourceFile("//foo/input3.o"));
  target.sources().push_back(SourceFile("//foo/input4.obj"));
  target.source_types_used().Set(SourceFile::SOURCE_CPP);
  target.source_types_used().Set(SourceFile::SOURCE_O);
  target.SetToolchain(setup.toolchain());
  ASSERT_TRUE(target.OnResolved(&err));

  StringOutputBuffer out;
  NinjaBinaryTargetWriter writer(&target, out);
  writer.Run();

  const char expected[] =
      "defines =\n"
      "include_dirs =\n"
      "cflags =\n"
      "cflags_cc =\n"
      "root_out_dir = .\n"
      "target_gen_dir = gen/foo\n"
      "target_out_dir = obj/foo\n"
      "target_output_name = bar\n"
      "\n"
      "build obj/foo/bar.input1.o: cxx ../../foo/input1.cc\n"
      "  source_file_part = input1.cc\n"
      "  source_name_part = input1\n"
      "build obj/foo/bar.input2.o: cxx ../../foo/input2.cc\n"
      "  source_file_part = input2.cc\n"
      "  source_name_part = input2\n"
      "\n"
      "build phony/foo/bar.linkdeps: phony obj/foo/bar.input1.o "
      "obj/foo/bar.input2.o ../../foo/input3.o ../../foo/input4.obj\n"
      "build phony/foo/bar: phony phony/foo/bar.linkdeps\n";
  EXPECT_EQ(expected, out);
}

TEST_F(NinjaBinaryTargetWriterTest, NoSourcesSourceSet) {
  Err err;
  TestWithScope setup;

  Target target(setup.settings(), Label(SourceDir("//foo/"), "bar"));
  target.set_output_type(Target::SOURCE_SET);
  target.visibility().SetPublic();
  target.SetToolchain(setup.toolchain());
  ASSERT_TRUE(target.OnResolved(&err));

  StringOutputBuffer out;
  NinjaBinaryTargetWriter writer(&target, out);
  writer.Run();

  const char expected[] =
      "defines =\n"
      "include_dirs =\n"
      "root_out_dir = .\n"
      "target_gen_dir = gen/foo\n"
      "target_out_dir = obj/foo\n"
      "target_output_name = bar\n"
      "\n"
      "\n";
  EXPECT_EQ(expected, out);
}

TEST_F(NinjaBinaryTargetWriterTest, NoSourcesStaticLib) {
  Err err;
  TestWithScope setup;

  Target target(setup.settings(), Label(SourceDir("//foo/"), "bar"));
  target.set_output_type(Target::STATIC_LIBRARY);
  target.visibility().SetPublic();
  target.SetToolchain(setup.toolchain());
  ASSERT_TRUE(target.OnResolved(&err));

  StringOutputBuffer out;
  NinjaBinaryTargetWriter writer(&target, out);
  writer.Run();

  const char expected[] =
      "defines =\n"
      "include_dirs =\n"
      "root_out_dir = .\n"
      "target_gen_dir = gen/foo\n"
      "target_out_dir = obj/foo\n"
      "target_output_name = libbar\n"
      "\n"
      "\n"
      "build obj/foo/libbar.a: alink\n"
      "  arflags =\n"
      "  output_extension =\n"
      "  output_dir =\n";
  EXPECT_EQ(expected, out);
}

TEST_F(NinjaBinaryTargetWriterTest, Inputs) {
  Err err;
  TestWithScope setup;

  {
    Target target(setup.settings(), Label(SourceDir("//foo/"), "bar"));
    target.set_output_type(Target::SOURCE_SET);
    target.visibility().SetPublic();
    target.sources().push_back(SourceFile("//foo/source1.cc"));
    target.config_values().inputs().push_back(SourceFile("//foo/input1"));
    target.config_values().inputs().push_back(SourceFile("//foo/input2"));
    target.source_types_used().Set(SourceFile::SOURCE_CPP);
    target.SetToolchain(setup.toolchain());
    ASSERT_TRUE(target.OnResolved(&err));

    StringOutputBuffer out;
    NinjaBinaryTargetWriter writer(&target, out);
    writer.Run();

    const char expected[] =
        "defines =\n"
        "include_dirs =\n"
        "cflags =\n"
        "cflags_cc =\n"
        "root_out_dir = .\n"
        "target_gen_dir = gen/foo\n"
        "target_out_dir = obj/foo\n"
        "target_output_name = bar\n"
        "\n"
        "build obj/foo/bar.source1.o: cxx ../../foo/source1.cc | "
        "../../foo/input1 ../../foo/input2\n"
        "  source_file_part = source1.cc\n"
        "  source_name_part = source1\n"
        "\n"
        "build phony/foo/bar.linkdeps: phony obj/foo/bar.source1.o\n"
        "build phony/foo/bar: phony phony/foo/bar.linkdeps\n";
    EXPECT_EQ(expected, out);
  }

  {
    Target target(setup.settings(), Label(SourceDir("//foo/"), "bar"));
    target.set_output_type(Target::SOURCE_SET);
    target.visibility().SetPublic();
    target.sources().push_back(SourceFile("//foo/source1.cc"));
    target.sources().push_back(SourceFile("//foo/source2.cc"));
    target.config_values().inputs().push_back(SourceFile("//foo/input1"));
    target.config_values().inputs().push_back(SourceFile("//foo/input2"));
    target.source_types_used().Set(SourceFile::SOURCE_CPP);
    target.SetToolchain(setup.toolchain());
    ASSERT_TRUE(target.OnResolved(&err));

    StringOutputBuffer out;
    NinjaBinaryTargetWriter writer(&target, out);
    writer.Run();

    const char expected[] =
        "defines =\n"
        "include_dirs =\n"
        "cflags =\n"
        "cflags_cc =\n"
        "root_out_dir = .\n"
        "target_gen_dir = gen/foo\n"
        "target_out_dir = obj/foo\n"
        "target_output_name = bar\n"
        "\n"
        "build phony/foo/bar.inputs: phony "
        "../../foo/input1 ../../foo/input2\n"
        "build obj/foo/bar.source1.o: cxx ../../foo/source1.cc | "
        "phony/foo/bar.inputs\n"
        "  source_file_part = source1.cc\n"
        "  source_name_part = source1\n"
        "build obj/foo/bar.source2.o: cxx ../../foo/source2.cc | "
        "phony/foo/bar.inputs\n"
        "  source_file_part = source2.cc\n"
        "  source_name_part = source2\n"
        "\n"
        "build phony/foo/bar.linkdeps: phony obj/foo/bar.source1.o "
        "obj/foo/bar.source2.o\n"
        "build phony/foo/bar: phony phony/foo/bar.linkdeps\n";
    EXPECT_EQ(expected, out);
  }
}

TEST_F(NinjaBinaryTargetWriterTest, PublicInputs) {
  Err err;
  TestWithScope setup;

  // Action A has public_inputs.
  Target a(setup.settings(), Label(SourceDir("//foo/"), "a"));
  a.set_output_type(Target::ACTION);
  a.visibility().SetPublic();
  a.SetToolchain(setup.toolchain());
  a.action_values().set_script(SourceFile("//foo/script.py"));
  a.public_inputs().push_back(SourceFile("//foo/a.in"));

  // Source set B depends on A.
  Target b(setup.settings(), Label(SourceDir("//foo/"), "b"));
  b.set_output_type(Target::SOURCE_SET);
  b.visibility().SetPublic();
  b.sources().push_back(SourceFile("//foo/source1.cc"));
  b.source_types_used().Set(SourceFile::SOURCE_CPP);
  b.SetToolchain(setup.toolchain());
  b.private_deps().push_back(LabelTargetPair(&a));

  ASSERT_TRUE(a.OnResolved(&err));
  ASSERT_TRUE(b.OnResolved(&err));

  StringOutputBuffer out;
  NinjaBinaryTargetWriter writer(&b, out);
  writer.Run();

  // B's compile lines should depend on a.in (since it inherits it from A).
  // Because it only has one input (a.in), it is depended on directly instead
  // of via a phony target.
  const char expected[] =
      "defines =\n"
      "include_dirs =\n"
      "cflags =\n"
      "cflags_cc =\n"
      "root_out_dir = .\n"
      "target_gen_dir = gen/foo\n"
      "target_out_dir = obj/foo\n"
      "target_output_name = b\n"
      "\n"
      "build obj/foo/b.source1.o: cxx ../../foo/source1.cc | "
      "phony/foo/a.public_inputs || phony/foo/a\n"
      "  source_file_part = source1.cc\n"
      "  source_name_part = source1\n"
      "\n"
      "build phony/foo/b.linkdeps: phony obj/foo/b.source1.o || phony/foo/a\n"
      "build phony/foo/b: phony phony/foo/b.linkdeps\n";
  EXPECT_EQ(expected, out);
}

TEST_F(NinjaBinaryTargetWriterTest, WriteCompileFlags) {
  Err err;
  TestWithScope setup;

  Config nested(setup.settings(), setup.ParseLabel("//:nested"));
  nested.visibility().SetPublic();
  nested.own_values().defines() = {"COMMON", "NESTED"};
  nested.own_values().cflags() = {"-duplicate"};
  nested.own_values().rustflags() = {"--cfg=nested"};
  ASSERT_TRUE(nested.OnResolved(&err));

  Config local(setup.settings(), setup.ParseLabel("//:local"));
  local.visibility().SetPublic();
  local.own_values().defines() = {"LOCAL"};
  local.own_values().inputs() = {SourceFile("//foo/config.input")};
  local.own_values().externs() = {{"my_crate", LibFile("path/to/lib.rlib")}};
  SubstitutionPattern extra_out;
  ASSERT_TRUE(extra_out.Parse(
      Value(nullptr, "{{source_out_dir}}/{{source_name_part}}.dwo"), &err));
  local.own_values().c_additional_outputs() = {extra_out};
  local.configs().push_back(LabelConfigPair(&nested));
  ASSERT_TRUE(local.OnResolved(&err));

  Config public_config(setup.settings(), setup.ParseLabel("//:public"));
  public_config.visibility().SetPublic();
  public_config.own_values().defines() = {"PUBLIC"};
  public_config.own_values().include_dirs() = {SourceDir("//dep/include/")};
  public_config.own_values().cflags_cc() = {"-std=c++23"};
  public_config.own_values().lib_dirs() = {SourceDir("//dep/lib/")};
  public_config.own_values().libs() = {LibFile("dep_lib")};
  public_config.own_values().framework_dirs() = {
      SourceDir("//dep/Frameworks/")};
  public_config.own_values().frameworks() = {"CoreFoundation.framework"};
  public_config.own_values().weak_frameworks() = {"WeakThing.framework"};
  public_config.own_values().weak_libraries() = {"libweak.dylib"};
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
  target.config_values().include_dirs() = {SourceDir("//dep/include/"),
                                           SourceDir("//local/include/")};
  target.config_values().set_precompiled_header("pch.h");
  target.config_values().set_precompiled_source(SourceFile("//foo/pch.cc"));
  target.configs().push_back(LabelConfigPair(&local));
  target.private_deps().push_back(LabelTargetPair(&dep));
  ASSERT_TRUE(target.OnResolved(&err)) << err.message();

  StringOutputBuffer ninja_out;
  NinjaBinaryTargetWriter writer(&target, ninja_out);
  StringOutputBuffer json_out;
  writer.WriteCompileFlags(json_out);

  const char expected[] = R"*({
   "c_additional_outputs": [ "{{source_out_dir}}/{{source_name_part}}.dwo" ],
   "cflags": [ "-duplicate", "-duplicate" ],
   "cflags_cc": [ "-std=c++23" ],
   "defines": [ "TARGET", "COMMON", "LOCAL", "NESTED", "PUBLIC" ],
   "externs": {
      "my_crate": "path/to/lib.rlib"
   },
   "framework_dirs": [ "//dep/Frameworks/" ],
   "frameworks": [ "CoreFoundation.framework" ],
   "include_dirs": [ "//dep/include/", "//local/include/" ],
   "inputs": [ "//foo/config.input" ],
   "lib_dirs": [ "//dep/lib/" ],
   "libs": [ "dep_lib" ],
   "precompiled_header": "pch.h",
   "precompiled_source": "//foo/pch.cc",
   "rustflags": [ "--cfg=nested" ],
   "target": "//:library(//toolchain:default)",
   "toolchain": "//toolchain:default",
   "weak_frameworks": [ "WeakThing.framework" ],
   "weak_libraries": [ "libweak.dylib" ]
}
)*";
  EXPECT_EQ(expected, json_out);
}

TEST_F(NinjaBinaryTargetWriterTest, WriteCompileFlagsFileOnRun) {
  Err err;
  TestWithScope setup;
  base::ScopedTempDir temp_dir;
  ASSERT_TRUE(temp_dir.CreateUniqueTempDir());
  setup.build_settings()->SetRootPath(temp_dir.GetPath());

  Target target(setup.settings(), setup.ParseLabel("//foo:bar"));
  target.set_output_type(Target::SOURCE_SET);
  target.visibility().SetPublic();
  target.config_values().defines() = {"FOO=1"};
  target.set_write_compile_flags_output(
      OutputFile("gen/foo/bar_compile_flags.json"));
  target.SetToolchain(setup.toolchain());
  ASSERT_TRUE(target.OnResolved(&err));

  SourceFile written_source("//out/Debug/gen/foo/bar_compile_flags.json");
  scheduler().AddUnknownGeneratedInput(&target, written_source);

  StringOutputBuffer out;
  NinjaBinaryTargetWriter writer(&target, out);
  writer.Run();

  EXPECT_TRUE(scheduler().GetUnknownGeneratedInputs().empty());

  StringOutputBuffer expected_json;
  writer.WriteCompileFlags(expected_json);
  base::FilePath expected_path =
      setup.build_settings()->GetFullPath(written_source);
  EXPECT_TRUE(expected_json.ContentsEqual(expected_path));
}
