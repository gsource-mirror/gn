// Copyright (c) 2019 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/json_project_writer.h"

#include "base/command_line.h"
#include "base/strings/string_util.h"
#include "gn/commands.h"
#include "gn/substitution_list.h"
#include "gn/target.h"
#include "gn/test_with_scheduler.h"
#include "gn/test_with_scope.h"
#include "util/build_config.h"
#include "util/test/test.h"

using JSONWriter = TestWithScheduler;

TEST_F(JSONWriter, ActionWithResponseFile) {
  Err err;
  TestWithScope setup;

  Target target(setup.settings(), Label(SourceDir("//foo/"), "bar"));
  target.set_output_type(Target::ACTION);

  target.sources().push_back(SourceFile("//foo/source1.txt"));
  target.config_values().inputs().push_back(SourceFile("//foo/input1.txt"));
  target.action_values().set_script(SourceFile("//foo/script.py"));

  target.SetToolchain(setup.toolchain());
  ASSERT_TRUE(target.OnResolved(&err));

  // Make sure we get interesting substitutions for both the args and the
  // response file contents.
  target.action_values().args() =
      SubstitutionList::MakeForTest("{{response_file_name}}");
  target.action_values().rsp_file_contents() =
      SubstitutionList::MakeForTest("-j", "3");
  target.action_values().outputs() =
      SubstitutionList::MakeForTest("//out/Debug/output1.out");

  setup.build_settings()->SetPythonPath(
      base::FilePath(FILE_PATH_LITERAL("/usr/bin/python")));
  std::vector<const Target*> targets;
  targets.push_back(&target);
#if defined(OS_WIN)
  base::FilePath root_path =
      base::FilePath(FILE_PATH_LITERAL("c:/path/to/src"));
#else
  base::FilePath root_path = base::FilePath(FILE_PATH_LITERAL("/path/to/src"));
#endif
  setup.build_settings()->SetRootPath(root_path);
  g_scheduler->AddGenDependency(root_path.Append(FILE_PATH_LITERAL(".gn")));
  g_scheduler->AddGenDependency(
      root_path.Append(FILE_PATH_LITERAL("BUILD.gn")));
  g_scheduler->AddGenDependency(
      root_path.Append(FILE_PATH_LITERAL("build/BUILD.gn")));
  std::string out =
      JSONProjectWriter::RenderJSON(setup.build_settings(), targets);
#if defined(OS_WIN)
  base::ReplaceSubstringsAfterOffset(&out, 0, "\r\n", "\n");
#endif
  const char expected_json[] =
      R"_({
   "build_settings": {
      "build_dir": "//out/Debug/",
      "default_toolchain": "//toolchain:default",
      "gen_input_files": [ "//.gn", "//BUILD.gn", "//build/BUILD.gn" ],
)_"
#if defined(OS_WIN)
      "      \"root_path\": \"c:/path/to/src\"\n"
#else
      "      \"root_path\": \"/path/to/src\"\n"
#endif
      R"_(   },
   "gen_files": {
   },
   "targets": {
      "//foo:bar()": {
         "args": [ "{{response_file_name}}" ],
         "check_includes_strict": false,
         "data_deps": [  ],
         "defined_from": "",
         "deps": [  ],
         "deps_loc": "",
         "deps_origins": {

         },
         "externs": {

         },
         "inputs": [ "//foo/input1.txt" ],
         "metadata": {

         },
         "outputs": [ "//out/Debug/output1.out" ],
         "private_deps": [  ],
         "private_deps_ordered": [  ],
         "public": "*",
         "public_deps": [  ],
         "public_deps_loc": "",
         "public_deps_ordered": [  ],
         "public_deps_origins": {

         },
         "public_loc": "",
         "response_file_contents": [ "-j", "3" ],
         "script": "//foo/script.py",
         "sources": [ "//foo/source1.txt" ],
         "testonly": false,
         "toolchain": "",
         "type": "action",
         "visibility": [  ]
      }
   },
   "toolchains": {
      "//toolchain:default": {
         "alink": {
            "command": "ar {{output}} {{source}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}.a" ],
            "weak_framework_switch": "-weak_framework "
         },
         "cc": {
            "command": "cc {{source}} {{cflags}} {{cflags_c}} {{defines}} {{include_dirs}} -o {{output}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         },
         "compile_xcassets": {
            "command": "touch {{output}}"
         },
         "copy": {
            "command": "cp {{source}} {{output}}"
         },
         "copy_bundle_data": {
            "command": "cp {{source}} {{output}}"
         },
         "cxx": {
            "command": "c++ {{source}} {{cflags}} {{cflags_cc}} {{defines}} {{include_dirs}} {{module_deps}} -o {{output}}",
            "command_launcher": "launcher",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         },
         "cxx_module": {
            "command": "c++ {{source}} {{cflags}} {{cflags_cc}} {{defines}} {{include_dirs}} {{module_deps_no_self}} -o {{output}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.pcm" ],
            "weak_framework_switch": "-weak_framework "
         },
         "link": {
            "command": "ld -o {{target_output_name}} {{source}} {{ldflags}} {{libs}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{root_out_dir}}/{{target_output_name}}" ],
            "weak_framework_switch": "-weak_framework "
         },
         "objc": {
            "command": "objcc {{source}} {{cflags}} {{cflags_objc}} {{defines}} {{include_dirs}} -o {{output}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         },
         "objcxx": {
            "command": "objcxx {{source}} {{cflags}} {{cflags_objcc}} {{defines}} {{include_dirs}} -o {{output}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         },
         "rust_bin": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "outputs": [ "{{root_out_dir}}/{{crate_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_cdylib": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_dylib": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_macro": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_rlib": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".rlib",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_staticlib": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".a",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "solink": {
            "command": "ld -shared -o {{target_output_name}}.so {{inputs}} {{ldflags}} {{libs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "output_prefix": "lib",
            "outputs": [ "{{root_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-weak_framework "
         },
         "solink_module": {
            "command": "ld -bundle -o {{target_output_name}}.so {{inputs}} {{ldflags}} {{libs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "output_prefix": "lib",
            "outputs": [ "{{root_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-weak_framework "
         },
         "stamp": {
            "command": "touch {{output}}"
         },
         "swift": {
            "command": "swiftc --module-name {{module_name}} {{module_dirs}} {{inputs}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{target_gen_dir}}/{{target_output_name}}.h", "{{target_out_dir}}/{{module_name}}.swiftmodule" ],
            "partial_outputs": [ "{{target_out_dir}}/{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         }
      }
   }
}
)_";
  EXPECT_EQ(expected_json, out);
}

// Tests that a Rust executable exports its aliased_deps and the native files
// that rustc passes to the linker.
TEST_F(JSONWriter, RustAliasedDepsAndLinkInputs) {
  Err err;
  TestWithScope setup;
  // Describing a target's deps reads the command switches.
  commands::CommandSwitches::Init(
      base::CommandLine(base::CommandLine::NO_PROGRAM));

  Target lib(setup.settings(), Label(SourceDir("//foo/"), "lib"));
  lib.set_output_type(Target::RUST_LIBRARY);
  lib.visibility().SetPublic();
  SourceFile lib_rs("//foo/lib.rs");
  lib.sources().push_back(lib_rs);
  lib.source_types_used().Set(SourceFile::SOURCE_RS);
  lib.rust_values().set_crate_root(lib_rs);
  lib.rust_values().crate_name() = "lib";
  lib.SetToolchain(setup.toolchain());
  ASSERT_TRUE(lib.OnResolved(&err));

  Target staticlib(setup.settings(), Label(SourceDir("//foo/"), "static"));
  staticlib.set_output_type(Target::STATIC_LIBRARY);
  staticlib.visibility().SetPublic();
  staticlib.sources().push_back(SourceFile("//foo/static.cpp"));
  staticlib.source_types_used().Set(SourceFile::SOURCE_CPP);
  staticlib.SetToolchain(setup.toolchain());
  ASSERT_TRUE(staticlib.OnResolved(&err));

  Target exe(setup.settings(), Label(SourceDir("//foo/"), "bar"));
  exe.set_output_type(Target::EXECUTABLE);
  exe.visibility().SetPublic();
  SourceFile main_rs("//foo/main.rs");
  exe.sources().push_back(main_rs);
  exe.source_types_used().Set(SourceFile::SOURCE_RS);
  exe.rust_values().set_crate_root(main_rs);
  exe.rust_values().crate_name() = "bar";
  exe.rust_values().aliased_deps()[lib.label()] = "renamed";
  exe.private_deps().push_back(LabelTargetPair(&lib));
  exe.private_deps().push_back(LabelTargetPair(&staticlib));
  exe.SetToolchain(setup.toolchain());
  ASSERT_TRUE(exe.OnResolved(&err));

  std::vector<const Target*> targets = {&lib, &staticlib, &exe};
  std::string out =
      JSONProjectWriter::RenderJSON(setup.build_settings(), targets);
  EXPECT_NE(out.find(R"_("//foo:lib()": "renamed")_"), std::string::npos)
      << out;
  EXPECT_NE(
      out.find(
          R"_("rust_link_inputs": [ "//out/Debug/obj/foo/libstatic.a" ])_"),
      std::string::npos)
      << out;
  // An rlib links nothing, so it exports no link inputs.
  EXPECT_EQ(out.find("rust_link_inputs"), out.rfind("rust_link_inputs")) << out;
  commands::CommandSwitches::Set(commands::CommandSwitches());
}

// Tests that a target exports the Clang modules that {{module_deps}} passes
// to its compiles: its own and that of its dependency.
TEST_F(JSONWriter, ClangModules) {
  Err err;
  TestWithScope setup;
  commands::CommandSwitches::Init(
      base::CommandLine(base::CommandLine::NO_PROGRAM));

  Target lib(setup.settings(), Label(SourceDir("//foo/"), "lib"));
  lib.set_output_type(Target::SOURCE_SET);
  lib.set_module_name("foo_lib");
  lib.visibility().SetPublic();
  lib.sources().push_back(SourceFile("//foo/lib.modulemap"));
  lib.source_types_used().Set(SourceFile::SOURCE_MODULEMAP);
  lib.set_module_type(1 << Target::HAS_MODULEMAP);
  lib.SetToolchain(setup.toolchain());
  ASSERT_TRUE(lib.OnResolved(&err));

  Target user(setup.settings(), Label(SourceDir("//foo/"), "user"));
  user.set_output_type(Target::SOURCE_SET);
  user.visibility().SetPublic();
  user.sources().push_back(SourceFile("//foo/user.cc"));
  user.source_types_used().Set(SourceFile::SOURCE_CPP);
  user.private_deps().push_back(LabelTargetPair(&lib));
  user.SetToolchain(setup.toolchain());
  ASSERT_TRUE(user.OnResolved(&err));

  std::vector<const Target*> targets = {&lib, &user};
  std::string out =
      JSONProjectWriter::RenderJSON(setup.build_settings(), targets);
  const char kModule[] =
      R"_("modulemap": "//foo/lib.modulemap",
            "name": "foo_lib",
            "pcm": "//out/Debug/obj/foo/lib.lib.pcm")_";
  // The user loads the module file; the library also marks it as its own.
  size_t first = out.find(kModule);
  ASSERT_NE(first, std::string::npos) << out;
  EXPECT_NE(out.find(kModule, first + 1), std::string::npos) << out;
  EXPECT_NE(out.find(R"_("self": true)_"), std::string::npos) << out;
  EXPECT_EQ(out.find(R"_("self": true)_"), out.rfind(R"_("self": true)_"))
      << out;
  commands::CommandSwitches::Set(commands::CommandSwitches());
}

TEST_F(JSONWriter, RustTarget) {
  Err err;
  TestWithScope setup;

  Target target(setup.settings(), Label(SourceDir("//foo/"), "bar"));
  target.set_output_type(Target::RUST_LIBRARY);
  target.visibility().SetPublic();
  SourceFile lib("//foo/lib.rs");
  target.sources().push_back(lib);
  target.source_types_used().Set(SourceFile::SOURCE_RS);
  target.rust_values().set_crate_root(lib);
  target.rust_values().crate_name() = "foo";
  target.SetToolchain(setup.toolchain());
  ASSERT_TRUE(target.OnResolved(&err));

  std::vector<const Target*> targets;
  targets.push_back(&target);
  std::string out =
      JSONProjectWriter::RenderJSON(setup.build_settings(), targets);
#if defined(OS_WIN)
  base::ReplaceSubstringsAfterOffset(&out, 0, "\r\n", "\n");
#endif
  const char expected_json[] =
      R"_({
   "build_settings": {
      "build_dir": "//out/Debug/",
      "default_toolchain": "//toolchain:default",
      "gen_input_files": [  ],
      "root_path": ""
   },
   "gen_files": {
   },
   "targets": {
      "//foo:bar()": {
         "allow_circular_includes_from": [  ],
         "check_includes": true,
         "check_includes_strict": false,
         "crate_name": "foo",
         "crate_root": "//foo/lib.rs",
         "data_deps": [  ],
         "defined_from": "",
         "deps": [  ],
         "deps_loc": "",
         "deps_origins": {

         },
         "externs": {

         },
         "metadata": {

         },
         "outputs": [ "//out/Debug/obj/foo/libbar.rlib" ],
         "private_deps": [  ],
         "private_deps_ordered": [  ],
         "public": "*",
         "public_deps": [  ],
         "public_deps_loc": "",
         "public_deps_ordered": [  ],
         "public_deps_origins": {

         },
         "public_loc": "",
         "sources": [ "//foo/lib.rs" ],
         "testonly": false,
         "toolchain": "",
         "type": "rust_library",
         "visibility": [ "*" ]
      }
   },
   "toolchains": {
      "//toolchain:default": {
         "alink": {
            "command": "ar {{output}} {{source}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}.a" ],
            "weak_framework_switch": "-weak_framework "
         },
         "cc": {
            "command": "cc {{source}} {{cflags}} {{cflags_c}} {{defines}} {{include_dirs}} -o {{output}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         },
         "compile_xcassets": {
            "command": "touch {{output}}"
         },
         "copy": {
            "command": "cp {{source}} {{output}}"
         },
         "copy_bundle_data": {
            "command": "cp {{source}} {{output}}"
         },
         "cxx": {
            "command": "c++ {{source}} {{cflags}} {{cflags_cc}} {{defines}} {{include_dirs}} {{module_deps}} -o {{output}}",
            "command_launcher": "launcher",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         },
         "cxx_module": {
            "command": "c++ {{source}} {{cflags}} {{cflags_cc}} {{defines}} {{include_dirs}} {{module_deps_no_self}} -o {{output}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.pcm" ],
            "weak_framework_switch": "-weak_framework "
         },
         "link": {
            "command": "ld -o {{target_output_name}} {{source}} {{ldflags}} {{libs}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{root_out_dir}}/{{target_output_name}}" ],
            "weak_framework_switch": "-weak_framework "
         },
         "objc": {
            "command": "objcc {{source}} {{cflags}} {{cflags_objc}} {{defines}} {{include_dirs}} -o {{output}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         },
         "objcxx": {
            "command": "objcxx {{source}} {{cflags}} {{cflags_objcc}} {{defines}} {{include_dirs}} -o {{output}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         },
         "rust_bin": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "outputs": [ "{{root_out_dir}}/{{crate_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_cdylib": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_dylib": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_macro": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_rlib": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".rlib",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_staticlib": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".a",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "solink": {
            "command": "ld -shared -o {{target_output_name}}.so {{inputs}} {{ldflags}} {{libs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "output_prefix": "lib",
            "outputs": [ "{{root_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-weak_framework "
         },
         "solink_module": {
            "command": "ld -bundle -o {{target_output_name}}.so {{inputs}} {{ldflags}} {{libs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "output_prefix": "lib",
            "outputs": [ "{{root_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-weak_framework "
         },
         "stamp": {
            "command": "touch {{output}}"
         },
         "swift": {
            "command": "swiftc --module-name {{module_name}} {{module_dirs}} {{inputs}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{target_gen_dir}}/{{target_output_name}}.h", "{{target_out_dir}}/{{module_name}}.swiftmodule" ],
            "partial_outputs": [ "{{target_out_dir}}/{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         }
      }
   }
}
)_";
  EXPECT_EQ(expected_json, out);
}

TEST_F(JSONWriter, ForEachWithResponseFile) {
  Err err;
  TestWithScope setup;

  Target target(setup.settings(), Label(SourceDir("//foo/"), "bar"));
  target.set_output_type(Target::ACTION_FOREACH);

  target.sources().push_back(SourceFile("//foo/input1.txt"));
  target.action_values().set_script(SourceFile("//foo/script.py"));

  target.SetToolchain(setup.toolchain());
  ASSERT_TRUE(target.OnResolved(&err));

  // Make sure we get interesting substitutions for both the args and the
  // response file contents.
  target.action_values().args() = SubstitutionList::MakeForTest(
      "{{source}}", "{{source_file_part}}", "{{response_file_name}}");
  target.action_values().rsp_file_contents() =
      SubstitutionList::MakeForTest("-j", "{{source_name_part}}");
  target.action_values().outputs() =
      SubstitutionList::MakeForTest("//out/Debug/{{source_name_part}}.out");

  setup.build_settings()->SetPythonPath(
      base::FilePath(FILE_PATH_LITERAL("/usr/bin/python")));
  std::vector<const Target*> targets;
  targets.push_back(&target);
#if defined(OS_WIN)
  base::FilePath root_path =
      base::FilePath(FILE_PATH_LITERAL("c:/path/to/src"));
#else
  base::FilePath root_path = base::FilePath(FILE_PATH_LITERAL("/path/to/src"));
#endif
  setup.build_settings()->SetRootPath(root_path);
  g_scheduler->AddGenDependency(root_path.Append(FILE_PATH_LITERAL(".gn")));
  g_scheduler->AddGenDependency(
      root_path.Append(FILE_PATH_LITERAL("BUILD.gn")));
  std::string out =
      JSONProjectWriter::RenderJSON(setup.build_settings(), targets);
#if defined(OS_WIN)
  base::ReplaceSubstringsAfterOffset(&out, 0, "\r\n", "\n");
#endif
  const char expected_json[] =
      R"_({
   "build_settings": {
      "build_dir": "//out/Debug/",
      "default_toolchain": "//toolchain:default",
      "gen_input_files": [ "//.gn", "//BUILD.gn" ],
)_"
#if defined(OS_WIN)
      "      \"root_path\": \"c:/path/to/src\"\n"
#else
      "      \"root_path\": \"/path/to/src\"\n"
#endif
      R"_(   },
   "gen_files": {
   },
   "targets": {
      "//foo:bar()": {
         "args": [ "{{source}}", "{{source_file_part}}", "{{response_file_name}}" ],
         "check_includes_strict": false,
         "data_deps": [  ],
         "defined_from": "",
         "deps": [  ],
         "deps_loc": "",
         "deps_origins": {

         },
         "externs": {

         },
         "metadata": {

         },
         "output_patterns": [ "//out/Debug/{{source_name_part}}.out" ],
         "outputs": [ "//out/Debug/input1.out" ],
         "private_deps": [  ],
         "private_deps_ordered": [  ],
         "public": "*",
         "public_deps": [  ],
         "public_deps_loc": "",
         "public_deps_ordered": [  ],
         "public_deps_origins": {

         },
         "public_loc": "",
         "response_file_contents": [ "-j", "{{source_name_part}}" ],
         "script": "//foo/script.py",
         "source_outputs": {
            "//foo/input1.txt": [ "input1.out" ]
         },
         "sources": [ "//foo/input1.txt" ],
         "testonly": false,
         "toolchain": "",
         "type": "action_foreach",
         "visibility": [  ]
      }
   },
   "toolchains": {
      "//toolchain:default": {
         "alink": {
            "command": "ar {{output}} {{source}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}.a" ],
            "weak_framework_switch": "-weak_framework "
         },
         "cc": {
            "command": "cc {{source}} {{cflags}} {{cflags_c}} {{defines}} {{include_dirs}} -o {{output}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         },
         "compile_xcassets": {
            "command": "touch {{output}}"
         },
         "copy": {
            "command": "cp {{source}} {{output}}"
         },
         "copy_bundle_data": {
            "command": "cp {{source}} {{output}}"
         },
         "cxx": {
            "command": "c++ {{source}} {{cflags}} {{cflags_cc}} {{defines}} {{include_dirs}} {{module_deps}} -o {{output}}",
            "command_launcher": "launcher",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         },
         "cxx_module": {
            "command": "c++ {{source}} {{cflags}} {{cflags_cc}} {{defines}} {{include_dirs}} {{module_deps_no_self}} -o {{output}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.pcm" ],
            "weak_framework_switch": "-weak_framework "
         },
         "link": {
            "command": "ld -o {{target_output_name}} {{source}} {{ldflags}} {{libs}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{root_out_dir}}/{{target_output_name}}" ],
            "weak_framework_switch": "-weak_framework "
         },
         "objc": {
            "command": "objcc {{source}} {{cflags}} {{cflags_objc}} {{defines}} {{include_dirs}} -o {{output}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         },
         "objcxx": {
            "command": "objcxx {{source}} {{cflags}} {{cflags_objcc}} {{defines}} {{include_dirs}} -o {{output}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{source_out_dir}}/{{target_output_name}}.{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         },
         "rust_bin": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "outputs": [ "{{root_out_dir}}/{{crate_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_cdylib": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_dylib": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_macro": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_rlib": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".rlib",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "rust_staticlib": {
            "command": "{{rustenv}} rustc --crate-name {{crate_name}} {{source}} --crate-type {{crate_type}} {{rustflags}} -o {{output}} {{rustdeps}} {{externs}}",
            "default_output_extension": ".a",
            "framework_dir_switch": "-Lframework=",
            "framework_switch": "-lframework=",
            "lib_dir_switch": "-Lnative=",
            "lib_switch": "-l",
            "linker_arg": "-Clink-arg=",
            "output_prefix": "lib",
            "outputs": [ "{{target_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-lframework="
         },
         "solink": {
            "command": "ld -shared -o {{target_output_name}}.so {{inputs}} {{ldflags}} {{libs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "output_prefix": "lib",
            "outputs": [ "{{root_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-weak_framework "
         },
         "solink_module": {
            "command": "ld -bundle -o {{target_output_name}}.so {{inputs}} {{ldflags}} {{libs}}",
            "default_output_extension": ".so",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "output_prefix": "lib",
            "outputs": [ "{{root_out_dir}}/{{target_output_name}}{{output_extension}}" ],
            "weak_framework_switch": "-weak_framework "
         },
         "stamp": {
            "command": "touch {{output}}"
         },
         "swift": {
            "command": "swiftc --module-name {{module_name}} {{module_dirs}} {{inputs}}",
            "framework_dir_switch": "-F",
            "framework_switch": "-framework ",
            "lib_dir_switch": "-L",
            "lib_switch": "-l",
            "outputs": [ "{{target_gen_dir}}/{{target_output_name}}.h", "{{target_out_dir}}/{{module_name}}.swiftmodule" ],
            "partial_outputs": [ "{{target_out_dir}}/{{source_name_part}}.o" ],
            "weak_framework_switch": "-weak_framework "
         }
      }
   }
}
)_";
  EXPECT_EQ(expected_json, out);
}

TEST_F(JSONWriter, FilterTargetsWithDataDeps) {
  Err err;
  TestWithScope setup;

  // Create targets :a, :b, :c
  Target target_a(setup.settings(), Label(SourceDir("//foo/"), "a"));
  target_a.set_output_type(Target::STATIC_LIBRARY);
  target_a.visibility().SetPublic();

  Target target_b(setup.settings(), Label(SourceDir("//foo/"), "b"));
  target_b.set_output_type(Target::STATIC_LIBRARY);
  target_b.visibility().SetPublic();

  Target target_c(setup.settings(), Label(SourceDir("//foo/"), "c"));
  target_c.set_output_type(Target::STATIC_LIBRARY);
  target_c.visibility().SetPublic();

  // :a depends on :b, and data_depends on :c
  target_a.private_deps().push_back(LabelTargetPair(&target_b));
  target_a.data_deps().push_back(LabelTargetPair(&target_c));

  target_a.SetToolchain(setup.toolchain());
  target_b.SetToolchain(setup.toolchain());
  target_c.SetToolchain(setup.toolchain());

  ASSERT_TRUE(target_a.OnResolved(&err));
  ASSERT_TRUE(target_b.OnResolved(&err));
  ASSERT_TRUE(target_c.OnResolved(&err));

  std::vector<const Target*> all_targets = {&target_a, &target_b, &target_c};
  std::vector<const Target*> filtered;

  // Only DEPS_LINKED (should include :a and :b, but not :c)
  ASSERT_TRUE(JSONProjectWriter::FilterTargets(
      setup.build_settings(), all_targets, &filtered, "//foo:a",
      /*filter_with_data_deps=*/false, &err));
  std::set<Label> labels_linked;
  for (const Target* t : filtered)
    labels_linked.insert(t->label());
  EXPECT_GT(labels_linked.count(Label(SourceDir("//foo/"), "a")), 0u);
  EXPECT_GT(labels_linked.count(Label(SourceDir("//foo/"), "b")), 0u);
  EXPECT_EQ(labels_linked.count(Label(SourceDir("//foo/"), "c")), 0u);

  // DEPS_ALL (should include :a, :b, :c)
  ASSERT_TRUE(JSONProjectWriter::FilterTargets(
      setup.build_settings(), all_targets, &filtered, "//foo:a",
      /*filter_with_data_deps=*/true, &err));
  std::set<Label> labels_all;
  for (const Target* t : filtered)
    labels_all.insert(t->label());
  EXPECT_GT(labels_all.count(Label(SourceDir("//foo/"), "a")), 0u);
  EXPECT_GT(labels_all.count(Label(SourceDir("//foo/"), "b")), 0u);
  EXPECT_GT(labels_all.count(Label(SourceDir("//foo/"), "c")), 0u);
}

TEST_F(JSONWriter, GroupWithData) {
  Err err;
  TestWithScope setup;

  Target target(setup.settings(), Label(SourceDir("//foo/"), "docs"));
  target.set_output_type(Target::GROUP);
  target.data().push_back("README.md");
  target.data().push_back("docs/help.txt");
  target.SetToolchain(setup.toolchain());
  ASSERT_TRUE(target.OnResolved(&err));

  std::vector<const Target*> targets = {&target};
  std::string out =
      JSONProjectWriter::RenderJSON(setup.build_settings(), targets);
#if defined(OS_WIN)
  base::ReplaceSubstringsAfterOffset(&out, 0, "\r\n", "\n");
#endif
  const char expected_json[] =
      R"_({
   "build_settings": {
      "build_dir": "//out/Debug/",
      "default_toolchain": "//toolchain:default",
      "gen_input_files": [  ],
      "root_path": ""
   },
   "gen_files": {
   },
   "targets": {
      "//foo:docs()": {
         "check_includes_strict": false,
         "data": [ "README.md", "docs/help.txt" ],
         "data_deps": [  ],
         "defined_from": "",
         "deps": [  ],
         "deps_loc": "",
         "deps_origins": {

         },
         "metadata": {

         },
         "private_deps": [  ],
         "private_deps_ordered": [  ],
         "public": "*",
         "public_deps": [  ],
         "public_deps_loc": "",
         "public_deps_ordered": [  ],
         "public_deps_origins": {

         },
         "public_loc": "",
         "testonly": false,
         "toolchain": "",
         "type": "group",
         "visibility": [  ]
      }
   },
   "toolchains": {
)_";

  EXPECT_TRUE(out.starts_with(expected_json)) << out;
}
