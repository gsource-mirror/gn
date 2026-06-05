// Copyright (c) 2013 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/scope.h"
#include "gn/test_with_scheduler.h"
#include "gn/test_with_scope.h"
#include "gn/parse_tree.h"
#include "base/files/file_path.h"
#include "base/files/file_util.h"
#include "base/files/scoped_temp_dir.h"
#include "util/test/test.h"

using FunctionsTarget = TestWithScheduler;

// Checks that we find unused identifiers in targets.
TEST_F(FunctionsTarget, CheckUnused) {
  TestWithScope setup;

  // The target generator needs a place to put the targets or it will fail.
  Scope::ItemVector item_collector;
  setup.scope()->set_item_collector(&item_collector);

  // Test a good one first.
  TestParseInput good_input(
      "source_set(\"foo\") {\n"
      "}\n");
  ASSERT_FALSE(good_input.has_error());
  Err err;
  good_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_SUCCESS(err);

  // Test a source set with an unused variable.
  TestParseInput source_set_input(
      "source_set(\"foo\") {\n"
      "  unused = 5\n"
      "}\n");
  ASSERT_FALSE(source_set_input.has_error());
  err = Err();
  source_set_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_TRUE(err.has_error());
}

// Checks that we find uses of identifiers marked as not needed.
TEST_F(FunctionsTarget, CheckNotNeeded) {
  TestWithScope setup;

  // The target generator needs a place to put the targets or it will fail.
  Scope::ItemVector item_collector;
  setup.scope()->set_item_collector(&item_collector);

  TestParseInput nonscoped_input(
      "source_set(\"foo\") {\n"
      "  a = 1\n"
      "  not_needed([ \"a\" ])\n"
      "}\n");
  ASSERT_FALSE(nonscoped_input.has_error());
  Err err;
  nonscoped_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_SUCCESS(err);

  TestParseInput scoped_input(
      "source_set(\"foo\") {\n"
      "  a = {x = 1 y = 2}\n"
      "  not_needed(a, \"*\")\n"
      "}\n");
  ASSERT_FALSE(scoped_input.has_error());
  err = Err();
  scoped_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_SUCCESS(err);

  TestParseInput nonexistent_arg_input(
      "source_set(\"foo\") {\n"
      "  a = {x = 1}\n"
      "  not_needed(a, [ \"x\", \"y\" ])\n"
      "}\n");
  ASSERT_FALSE(nonexistent_arg_input.has_error());
  err = Err();
  nonexistent_arg_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_SUCCESS(err);

  TestParseInput exclusion_input(
      "source_set(\"foo\") {\n"
      "  x = 1\n"
      "  y = 2\n"
      "  not_needed(\"*\", [ \"y\" ])\n"
      "}\n");
  ASSERT_FALSE(exclusion_input.has_error());
  err = Err();
  exclusion_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_TRUE(err.has_error()) << err.message();
  EXPECT_EQ("Assignment had no effect.", err.message());

  TestParseInput error_input(
      "source_set(\"foo\") {\n"
      "  a = {x = 1 y = 2}\n"
      "  not_needed(a, [ \"x \"], [ \"y\" ])\n"
      "}\n");
  ASSERT_FALSE(error_input.has_error());
  err = Err();
  error_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_TRUE(err.has_error());
  EXPECT_EQ("Not supported with a variable list.", err.message());

  TestParseInput argcount_error_input(
      "source_set(\"foo\") {\n"
      "  not_needed()\n"
      "}\n");
  ASSERT_FALSE(argcount_error_input.has_error());
  err = Err();
  argcount_error_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_TRUE(err.has_error());
  EXPECT_EQ("Wrong number of arguments.", err.message());

  TestParseInput scope_error_input(
      "source_set(\"foo\") {\n"
      "  a = {x = 1 y = 2}\n"
      "  not_needed(a)\n"
      "}\n");
  ASSERT_FALSE(scope_error_input.has_error());
  err = Err();
  scope_error_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_TRUE(err.has_error());
  EXPECT_EQ("Wrong number of arguments.", err.message());

  TestParseInput string_error_input(
      "source_set(\"foo\") {\n"
      "  not_needed(\"*\", {}, \"*\")\n"
      "}\n");
  ASSERT_FALSE(string_error_input.has_error());
  err = Err();
  string_error_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_TRUE(err.has_error());
  EXPECT_EQ("Wrong number of arguments.", err.message());

  TestParseInput template_input(
      R"(# Test that not_needed() propagates through templates correctly;
      # no error should arise from not using "a".
      template("inner_templ") {
        source_set(target_name) {
          not_needed(invoker, [ "a" ])
        }
      }
      template("outer_templ") {
        inner_templ(target_name) {
          forward_variables_from(invoker, "*")
        }
      }
      outer_templ("foo") {
        a = 1
      })");
  ASSERT_FALSE(template_input.has_error());
  err = Err();
  template_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_SUCCESS(err);
}

// Checks that the defaults applied to a template invoked by target() use
// the name of the template, rather than the string "target" (which is the
// name of the actual function being called).
TEST_F(FunctionsTarget, TemplateDefaults) {
  TestWithScope setup;

  // The target generator needs a place to put the targets or it will fail.
  Scope::ItemVector item_collector;
  setup.scope()->set_item_collector(&item_collector);

  // Test a good one first.
  TestParseInput good_input(
      R"(# Make a template with defaults set.
      template("my_templ") {
        source_set(target_name) {
          forward_variables_from(invoker, "*")
        }
      }
      set_defaults("my_templ") {
        default_value = 1
      }

      # Invoke the template with target(). This will fail to execute if the
      # defaults were not set properly, because "default_value" won't exist.
      target("my_templ", "foo") {
        print(default_value)
      })");
  ASSERT_FALSE(good_input.has_error());
  Err err;
  good_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_SUCCESS(err);
}

// Checks that we find unused identifiers in targets.
TEST_F(FunctionsTarget, MixedSourceError) {
  TestWithScope setup;

  // The target generator needs a place to put the targets or it will fail.
  Scope::ItemVector item_collector;
  setup.scope()->set_item_collector(&item_collector);

  // Test a good one first.
  TestParseInput good_input(
      "source_set(\"foo\") {\n"
      "  sources = [ \"cpp.cc\", \"rust.rs\" ]"
      "}\n");
  ASSERT_FALSE(good_input.has_error());
  Err err;
  good_input.parsed()->Execute(setup.scope(), &err);
  ASSERT_TRUE(err.has_error());
  ASSERT_EQ(err.message(), "More than one language used in target sources.");
}

TEST_F(FunctionsTarget, StarlarkRuleTarget) {
  TestWithScope setup;

  // The target generator needs a place to put the targets or it will fail.
  Scope::ItemVector item_collector;
  setup.scope()->set_item_collector(&item_collector);

  base::ScopedTempDir temp_dir;
  ASSERT_TRUE(temp_dir.CreateUniqueTempDir());
  setup.build_settings()->SetRootPath(temp_dir.GetPath());

  std::string bzl_content = R"bzl(
def _impl(ctx):
  if ctx.attr.some_val == 42:
    fail("expected failure for 42")

my_rule = rule_extension(
  implementation = _impl,
  attrs = {
    "some_val": attr.int(default = 1),
  },
)
)bzl";

  base::FilePath bzl_path = temp_dir.GetPath().AppendASCII("rules.bzl");
  ASSERT_EQ(static_cast<int>(bzl_content.size()),
            base::WriteFile(bzl_path, bzl_content.c_str(), bzl_content.size()));

  // 1. Success case: some_val = 10
  {
    TestParseInput input(R"gn(
      load("//:rules.bzl", "my_rule")
      static_library = my_rule
      static_library("target_ok") {
        some_val = 10
      }
    )gn");
    ASSERT_SUCCESS(input);

    Err err;
    input.parsed()->Execute(setup.scope(), &err);
    ASSERT_FALSE(err.has_error()) << err.message() << "\n" << err.help_text();

    ASSERT_EQ(1u, item_collector.size());
    Target* target = item_collector[0]->AsTarget();
    ASSERT_TRUE(target);

    target->SetToolchain(setup.toolchain());
    Err resolve_err;
    ASSERT_TRUE(target->OnResolvedWithoutChecks(&resolve_err));

    Err callback_err;
    bool callback_ok = target->run_starlark_rule_impl(&callback_err);
    ASSERT_FALSE(callback_err.has_error()) << callback_err.message();
    ASSERT_TRUE(callback_ok);

    item_collector.clear();
  }

  // 2. Failure case: some_val = 42
  {
    TestParseInput input(R"gn(
      load("//:rules.bzl", "my_rule")
      static_library = my_rule
      static_library("target_fail") {
        some_val = 42
      }
    )gn");
    ASSERT_SUCCESS(input);

    Err err;
    input.parsed()->Execute(setup.scope(), &err);
    ASSERT_FALSE(err.has_error()) << err.message() << "\n" << err.help_text();

    ASSERT_EQ(1u, item_collector.size());
    Target* target = item_collector[0]->AsTarget();
    ASSERT_TRUE(target);

    target->SetToolchain(setup.toolchain());
    Err resolve_err;
    ASSERT_TRUE(target->OnResolvedWithoutChecks(&resolve_err));

    Err callback_err;
    bool callback_ok = target->run_starlark_rule_impl(&callback_err);
    ASSERT_FALSE(callback_ok);
    ASSERT_TRUE(callback_err.has_error());

    item_collector.clear();
  }
}

