// Copyright 2020 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/string_output_buffer.h"

#include <algorithm>
#include <limits>

#include "base/files/file_path.h"
#include "base/files/file_util.h"
#include "base/files/scoped_temp_dir.h"
#include "third_party/zstd/lib/zstd.h"
#include "util/test/test.h"

namespace {

// Create a test string of |size| characters with pseudo-random ASCII content.
std::string CreateTestString(size_t size, size_t seed = 0) {
  std::string result;
  result.resize(size);
  for (size_t n = 0; n < size; ++n) {
    int offset = (size + seed + n * 1337);
    char ch = ' ' + offset % (127 - 32);
    result[n] = ch;
  }
  return result;
}

}  // namespace

TEST(StringOutputBuffer, Append) {
  const size_t data_size = 100000;
  std::string data = CreateTestString(data_size);

  const size_t num_spans = 50;
  const size_t span_size = data_size / num_spans;

  StringOutputBuffer buffer;

  for (size_t n = 0; n < num_spans; ++n) {
    size_t start_offset = n * span_size;
    size_t end_offset = std::min(start_offset + span_size, data.size());
    buffer.Append(&data[start_offset], end_offset - start_offset);
  }

  EXPECT_EQ(data.size(), buffer.size());
  ASSERT_STREQ(data.c_str(), buffer.str().c_str());
}

TEST(StringOutputBuffer, AppendWithPageSizeMultiples) {
  const size_t page_size = StringOutputBuffer::GetPageSizeForTesting();
  const size_t page_count = 100;
  const size_t data_size = page_size * page_count;
  std::string data = CreateTestString(data_size);

  StringOutputBuffer buffer;

  for (size_t n = 0; n < page_count; ++n) {
    size_t start_offset = n * page_size;
    buffer.Append(&data[start_offset], page_size);
  }

  EXPECT_EQ(data.size(), buffer.size());
  ASSERT_STREQ(data.c_str(), buffer.str().c_str());
}

TEST(StringOutputBuffer, Clear) {
  StringOutputBuffer buffer;
  buffer << "first string";
  EXPECT_EQ("first string", buffer.str());
  EXPECT_EQ(12u, buffer.size());

  buffer.Clear();
  EXPECT_EQ("", buffer.str());
  EXPECT_EQ(0u, buffer.size());

  buffer << "second string";
  EXPECT_EQ("second string", buffer.str());
  EXPECT_EQ(13u, buffer.size());
}

TEST(StringOutputBuffer, ContentsEqual) {
  base::ScopedTempDir temp_dir;
  ASSERT_TRUE(temp_dir.CreateUniqueTempDir());

  const size_t data_size = 100000;
  std::string elem = CreateTestString(data_size);
  std::string data = elem + elem;

  base::FilePath file_path = temp_dir.GetPath().AppendASCII("foo.txt");
  base::WriteFile(file_path, data.c_str(), static_cast<int>(data.size()));

  {
    StringOutputBuffer buffer;
    // If we just call buffer.Append(data), then it remains in the same
    // allocation, and thus our code isn't tested properly.
    buffer.Append(elem);
    buffer.Append(elem);
    EXPECT_EQ(data, buffer);

    EXPECT_TRUE(buffer.ContentsEqual(file_path));

    // Different length and contents.
    buffer << "extra";
    EXPECT_FALSE(buffer.ContentsEqual(file_path));
  }

  // The same length, different contents.
  {
    StringOutputBuffer buffer;
    buffer << CreateTestString(data_size, 1);

    EXPECT_FALSE(buffer.ContentsEqual(file_path));
  }
}

TEST(StringOutputBuffer, WriteToFile) {
  base::ScopedTempDir temp_dir;
  ASSERT_TRUE(temp_dir.CreateUniqueTempDir());

  const size_t data_size = 100000;
  std::string data = CreateTestString(data_size);

  // Write if file doesn't exist. Also create directory.
  base::FilePath file_path =
      temp_dir.GetPath().AppendASCII("bar").AppendASCII("foo.txt");

  StringOutputBuffer buffer;
  buffer.Append(data);

  EXPECT_TRUE(buffer.WriteToFile(file_path, nullptr));

  // Verify file was created and has same content.
  base::File::Info file_info;
  ASSERT_TRUE(base::GetFileInfo(file_path, &file_info));
  ASSERT_TRUE(buffer.ContentsEqual(file_path));
}

// Ninja-like content with long binding blocks, "$" continuations (also with
// CRLF), comments and blank lines, none of which may start a frame.
std::string CreateNinjaTestString(int statements) {
  std::string data;
  for (int n = 0; n < statements; ++n) {
    std::string i = std::to_string(n);
    data += "build obj/a" + i + ".o: cxx ../../a" + i + ".cc $\n";
    data += "    ../../b" + i + ".h $\r\n";
    data += "    ../../c" + i + ".h\n";
    for (int b = 0; b < 8; ++b)
      data += "  defines = -DFOO" + std::to_string(b) + " -DBAR=" + i + "\n";
    data += "\n# comment " + i + "\n";
    data += "\tindented" + i + "\n";
    data += "#another comment\n\r\n";
  }
  return data;
}

// Siso's chunk boundary rule, written independently of the implementation.
bool IsStatementStart(std::string_view s, size_t i) {
  if (i == 0 || i >= s.size() || s[i - 1] != '\n')
    return false;
  if (i >= 2 && s[i - 2] == '$')
    return false;
  if (i >= 3 && s[i - 3] == '$' && s[i - 2] == '\r')
    return false;
  return s[i] != ' ' && s[i] != '\t' && s[i] != '#' && s[i] != '\r' &&
         s[i] != '\n';
}

// Checks that CompressZstd(frame_size) of |data| decompresses to |data| and
// that each frame records its size and ends at the first statement start after
// |frame_size| bytes. Returns the number of frames.
size_t CheckCompressZstd(std::string_view data, size_t frame_size) {
  // Tiny appends make the boundary checks straddle page edges.
  StringOutputBuffer buffer;
  for (size_t pos = 0; pos < data.size(); pos += 3)
    buffer.Append(data.substr(pos, 3));
  std::string compressed = buffer.CompressZstd(frame_size).str();

  std::vector<size_t> frame_ends;
  size_t end = 0;
  for (size_t pos = 0; pos < compressed.size();) {
    const char* frame = compressed.data() + pos;
    size_t n = ZSTD_findFrameCompressedSize(frame, compressed.size() - pos);
    EXPECT_FALSE(ZSTD_isError(n)) << ZSTD_getErrorName(n);
    if (ZSTD_isError(n))
      return 0;
    unsigned long long content_size = ZSTD_getFrameContentSize(frame, n);
    EXPECT_NE(ZSTD_CONTENTSIZE_UNKNOWN, content_size);
    end += content_size;
    frame_ends.push_back(end);
    pos += n;
  }
  std::vector<size_t> want_ends;
  for (size_t want_end = 0; want_end < data.size();) {
    want_end += frame_size;
    while (want_end < data.size() && !IsStatementStart(data, want_end))
      ++want_end;
    want_end = std::min(want_end, data.size());
    want_ends.push_back(want_end);
  }
  if (data.empty())
    want_ends.push_back(0);  // Empty content still takes one frame.
  EXPECT_EQ(want_ends, frame_ends) << frame_size;

  std::string decompressed(data.size(), '\0');
  size_t n = ZSTD_decompress(decompressed.data(), decompressed.size(),
                             compressed.data(), compressed.size());
  EXPECT_FALSE(ZSTD_isError(n)) << ZSTD_getErrorName(n);
  EXPECT_EQ(data, decompressed);

  // The frames don't depend on how the content is split into pages.
  StringOutputBuffer one_piece;
  one_piece.Append(data);
  EXPECT_EQ(compressed, one_piece.CompressZstd(frame_size).str());
  return frame_ends.size();
}

TEST(StringOutputBuffer, CompressZstd) {
  std::string data = CreateNinjaTestString(200);
  for (size_t frame_size : {size_t(1), size_t(64), size_t(100), size_t(777),
                            size_t(4096), StringOutputBuffer::kZstdFrameSize}) {
    size_t frames = CheckCompressZstd(data, frame_size);
    if (frame_size < data.size())
      EXPECT_GT(frames, 1u) << frame_size;
  }

  EXPECT_EQ(1u, CheckCompressZstd("", 64));

  // Without any later statement, the rest goes into the last frame.
  std::string no_statements = "build a: b $\n";
  for (int n = 0; n < 1000; ++n)
    no_statements += "  x = y\n# z\n\n";
  EXPECT_EQ(1u, CheckCompressZstd(no_statements, 64));
}

namespace {

struct CustomGreeting {
  std::string_view name;
};

struct Unwritable {};

size_t MaximumSize(const CustomGreeting& greeting) {
  return greeting.name.size() + 7;
}

char* WriteToBuf(const CustomGreeting& greeting, char* buf) {
  constexpr std::string_view prefix = "Hello, ";
  memcpy(buf, prefix.data(), prefix.size());
  buf += prefix.size();
  memcpy(buf, greeting.name.data(), greeting.name.size());
  return buf + greeting.name.size();
}

static_assert(Writable<CustomGreeting>);
static_assert(Writable<std::string_view>);
static_assert(Writable<char>);
static_assert(Writable<std::string>);
static_assert(Writable<const char*>);
static_assert(Writable<int>);
static_assert(Writable<int64_t>);
static_assert(Writable<uint64_t>);
static_assert(!Writable<bool>);
static_assert(!Writable<Unwritable>);

}  // namespace

TEST(StringOutputBuffer, CustomWritable) {
  StringOutputBuffer buffer;
  buffer << CustomGreeting{"world "} << std::numeric_limits<int>::min() << " "
         << std::numeric_limits<int>::max() << '!';
  EXPECT_EQ("Hello, world -2147483648 2147483647!", buffer);
}