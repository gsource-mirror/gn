// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/headers_map_writer.h"

#include "base/command_line.h"
#include "base/files/file_path.h"
#include "base/files/file_util.h"
#include "base/files/scoped_temp_dir.h"
#include "gn/commands.h"
#include "gn/filesystem_utils.h"
#include "gn/switches.h"
#include "gn/test_with_scheduler.h"
#include "util/test/test.h"

using HeadersMapWriterTest = TestWithScheduler;

TEST(HeadersMapWriterTest, GenerateFiles) {
  Label default_toolchain(SourceDir("//toolchain/"), "default");
  Label second_toolchain(SourceDir("//toolchain/"), "second");
  Label third_toolchain(SourceDir("//toolchain/"), "third");

  auto make_label = [](auto name, const Label& toolchain) {
    return Label(SourceDir("//"), name, toolchain.dir(), toolchain.name());
  };

  auto a = make_label("a", default_toolchain);
  auto a_second = make_label("a", second_toolchain);
  auto a_third = make_label("a", third_toolchain);
  auto b = make_label("b", default_toolchain);

  std::map<std::string_view, std::vector<const Label*>> header_to_targets = {
      {"two_labels.h", {&a, &b}},
      {"default_included.h", {&a, &a_second, &a_third}},
      {"default_not_included.h", {&a_second, &a_third}},
  };

  auto got =
      HeadersMapWriter::GenerateFiles(default_toolchain, header_to_targets)
          .str();

  std::string expected = R"##(default_included.h //:a
default_not_included.h //:a
two_labels.h //:a //:b
)##";

  EXPECT_EQ(got, expected) << got << "\n" << expected;
}
