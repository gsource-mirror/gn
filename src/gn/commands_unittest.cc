// Copyright 2014 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <stddef.h>

#include "base/command_line.h"
#include "base/values.h"

#include "gn/commands.h"
#include "gn/label_pattern.h"
#include "gn/target.h"
#include "gn/test_with_scope.h"
#include "util/test/test.h"

TEST(Commands, FilterOutMatch) {
  TestWithScope setup;
  SourceDir current_dir("//");

  Target target_afoo(setup.settings(), Label(SourceDir("//a/"), "foo"));
  Target target_cbar(setup.settings(), Label(SourceDir("//c/"), "bar"));
  std::vector<const Target*> targets{&target_afoo, &target_cbar};

  Err err;
  LabelPattern pattern_a = LabelPattern::GetPattern(
      current_dir, std::string_view(), Value(nullptr, "//a:*"), &err);
  EXPECT_SUCCESS(err);
  LabelPattern pattern_ef = LabelPattern::GetPattern(
      current_dir, std::string_view(), Value(nullptr, "//e:f"), &err);
  EXPECT_SUCCESS(err);
  std::vector<LabelPattern> label_patterns{pattern_a, pattern_ef};

  std::vector<const Target*> output;
  commands::FilterOutTargetsByPatterns(targets, label_patterns, &output);

  EXPECT_EQ(1, output.size());
  EXPECT_EQ(&target_cbar, output[0]);
}

TEST(Commands, ApplyTypeFilter) {
  TestWithScope setup;
  SourceDir current_dir("//");

  static constexpr std::pair<std::string, Target::OutputType> cases[] = {
      {"group", Target::GROUP},
      {"executable", Target::EXECUTABLE},
      {"shared_library", Target::SHARED_LIBRARY},
      {"loadable_module", Target::LOADABLE_MODULE},
      {"static_library", Target::STATIC_LIBRARY},
      {"source_set", Target::SOURCE_SET},
      {"copy", Target::COPY_FILES},
      {"action", Target::ACTION},
      {"generated_file", Target::GENERATED_FILE},
      {"rust_library", Target::RUST_LIBRARY},
      {"rust_proc_macro", Target::RUST_PROC_MACRO},
      {"bundle_data", Target::BUNDLE_DATA},
      {"create_bundle", Target::CREATE_BUNDLE},
  };

  std::vector<std::unique_ptr<Target>> created_targets;
  std::vector<const Target*> all_targets;

  for (const auto& test_case : cases) {
    auto target = std::make_unique<Target>(
        setup.settings(), Label(SourceDir("//a/"), test_case.first));
    target->set_output_type(test_case.second);
    all_targets.push_back(target.get());
    created_targets.push_back(std::move(target));
  }

  for (const auto& test_case : cases) {
    std::vector<const Target*> targets_to_filter = all_targets;

    base::CommandLine cmdline(base::CommandLine::NO_PROGRAM);
    cmdline.AppendSwitch("type", test_case.first);

    commands::CommandSwitches::Init(cmdline);

    base::ListValue out;
    commands::FilterAndPrintTargets(&targets_to_filter, &out);

    EXPECT_EQ(1u, targets_to_filter.size());
    EXPECT_EQ(test_case.second, targets_to_filter[0]->output_type())
        << "Failed for type: " << test_case.first;

    commands::CommandSwitches empty_switches;
    commands::CommandSwitches::Set(empty_switches);
  }
}
