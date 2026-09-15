// Copyright (c) 2013 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/escape.h"
#include "gn/string_output_buffer.h"
#include "util/test/test.h"

TEST(Escape, Ninja) {
  EscapeOptions opts;
  opts.mode = ESCAPE_NINJA;
  std::string result = EscapeString("asdf: \"$\\bar", opts, nullptr);
  EXPECT_EQ("asdf$:$ \"$$\\bar", result);
}

TEST(Escape, Depfile) {
  EscapeOptions opts;
  opts.mode = ESCAPE_DEPFILE;
  std::string result = EscapeString("asdf:$ \\#*[|]bar", opts, nullptr);
  EXPECT_EQ("asdf:$$\\ \\\\\\#\\*\\[\\|\\]bar", result);
}

TEST(Escape, WindowsCommand) {
  EscapeOptions opts;
  opts.mode = ESCAPE_NINJA_COMMAND;
  opts.platform = ESCAPE_PLATFORM_WIN;

  // Regular string is passed, even if it has backslashes.
  EXPECT_EQ("foo\\bar", EscapeString("foo\\bar", opts, nullptr));

  // Spaces means the string is quoted, normal backslahes untouched.
  bool needs_quoting = false;
  EXPECT_EQ("\"foo\\$ bar\"", EscapeString("foo\\ bar", opts, &needs_quoting));
  EXPECT_TRUE(needs_quoting);

  // Inhibit quoting.
  needs_quoting = false;
  opts.inhibit_quoting = true;
  EXPECT_EQ("foo\\$ bar", EscapeString("foo\\ bar", opts, &needs_quoting));
  EXPECT_TRUE(needs_quoting);
  opts.inhibit_quoting = false;

  // Backslashes at the end of the string get escaped.
  EXPECT_EQ("\"foo$ bar\\\\\\\\\"", EscapeString("foo bar\\\\", opts, nullptr));

  // Backslashes preceding quotes are escaped, and the quote is escaped.
  EXPECT_EQ("\"foo\\\\\\\"$ bar\"", EscapeString("foo\\\" bar", opts, nullptr));
}

TEST(Escape, PosixCommand) {
  EscapeOptions opts;
  opts.mode = ESCAPE_NINJA_COMMAND;
  opts.platform = ESCAPE_PLATFORM_POSIX;

  // : and $ ninja escaped with $. Then Shell-escape backslashes and quotes.
  EXPECT_EQ("a$:\\$ \\\"\\$$\\\\b", EscapeString("a: \"$\\b", opts, nullptr));

  // Some more generic shell chars.
  EXPECT_EQ("a_\\;\\<\\*b", EscapeString("a_;<*b", opts, nullptr));

  // Curly braces must be escaped to avoid brace expansion on systems using
  // bash as default shell..
  EXPECT_EQ("\\{a,b\\}\\{c,d\\}", EscapeString("{a,b}{c,d}", opts, nullptr));
}

TEST(Escape, NinjaPreformatted) {
  EscapeOptions opts;
  opts.mode = ESCAPE_NINJA_PREFORMATTED_COMMAND;

  // Only $ is escaped.
  EXPECT_EQ("a: \"$$\\b<;", EscapeString("a: \"$\\b<;", opts, nullptr));
}

TEST(Escape, Space) {
  EscapeOptions opts;
  opts.mode = ESCAPE_SPACE;

  // ' ' is escaped.
  EXPECT_EQ("-VERSION=\"libsrtp2\\ 2.1.0-pre\"",
            EscapeString("-VERSION=\"libsrtp2 2.1.0-pre\"", opts, nullptr));
}

TEST(EscapeJSONString, NinjaPreformatted) {
  EscapeOptions opts;
  opts.mode = ESCAPE_NINJA_PREFORMATTED_COMMAND;
  opts.inhibit_quoting = true;

  StringOutputBuffer buffer;
  std::ostream out(&buffer);

  EscapeJSONStringToStream(out, "foo\\\" bar", opts);
  EXPECT_EQ("foo\\\\\\\" bar", buffer.str());

  StringOutputBuffer buffer1;
  std::ostream out1(&buffer1);
  EscapeJSONStringToStream(out1, "foo bar\\\\", opts);
  EXPECT_EQ("foo bar\\\\\\\\", buffer1.str());

  StringOutputBuffer buffer2;
  std::ostream out2(&buffer2);
  EscapeJSONStringToStream(out2, "a: \"$\\b", opts);
  EXPECT_EQ("a: \\\"$$\\\\b", buffer2.str());
}

TEST(Escape, CompilationDatabase) {
  EscapeOptions opts;
  opts.mode = ESCAPE_COMPILATION_DATABASE;

  // The only special characters are '"' and '\'.
  std::string result = EscapeString("asdf:$ \\#*[|]bar", opts, nullptr);
  EXPECT_EQ("\"asdf:$ \\\\#*[|]bar\"", result);
}

TEST(Escape, NeedsEscapeNinja) {
  auto check = [](std::string_view str, bool expected) {
    EXPECT_EQ(expected, internal::NeedsEscapeNinjaScalar(str));
#if defined(__x86_64__) || defined(_M_X64) || defined(__i386__) || \
    defined(_M_IX86)
    EXPECT_EQ(expected, internal::NeedsEscapeNinjaSIMD(str));
#endif
  };

  // Empty string.
  check("", false);

  // Strings without special characters of various lengths.
  check("a", false);
  check("0123456789abcde", false);                    // 15 chars
  check("0123456789abcdef", false);                   // 16 chars
  check("0123456789abcdefg", false);                  // 17 chars
  check("0123456789abcdef0123456789abcde", false);    // 31 chars
  check("0123456789abcdef0123456789abcdef", false);   // 32 chars
  check("0123456789abcdef0123456789abcdefg", false);  // 33 chars
  check("obj/chrome/browser/ui/views/button.o", false);

  // Strings with special characters (' ', '$', ':') at various positions.
  const char special_chars[] = {' ', '$', ':'};
  for (char ch : special_chars) {
    // Single char.
    check(std::string(1, ch), true);

    // At the beginning.
    check(std::string(1, ch) + "abcdefghijklmnop", true);

    // At the end of various lengths.
    check(std::string(14, 'a') + ch, true);  // 15th char (index 14)
    check(std::string(15, 'a') + ch, true);  // 16th char (index 15)
    check(std::string(16, 'a') + ch, true);  // 17th char (index 16)
    check(std::string(31, 'a') + ch, true);  // 32nd char (index 31)
    check(std::string(32, 'a') + ch, true);  // 33rd char (index 32)
  }

#if defined(__x86_64__) || defined(_M_X64) || defined(__i386__) || \
    defined(_M_IX86)
  // Exhaustive comparison between Scalar and SIMD for lengths 0 to 64
  // and all possible single-character insertion positions.
  for (size_t len = 0; len <= 64; ++len) {
    std::string base(len, 'a');
    EXPECT_EQ(internal::NeedsEscapeNinjaScalar(base),
              internal::NeedsEscapeNinjaSIMD(base));

    for (char ch : special_chars) {
      for (size_t pos = 0; pos < len; ++pos) {
        std::string modified = base;
        modified[pos] = ch;
        bool scalar_res = internal::NeedsEscapeNinjaScalar(modified);
        bool simd_res = internal::NeedsEscapeNinjaSIMD(modified);
        EXPECT_TRUE(scalar_res);
        EXPECT_EQ(scalar_res, simd_res);
      }
    }
  }
#endif
}
