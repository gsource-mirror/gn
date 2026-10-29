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

TEST(StringOutputBuffer, CompressZstd) {
  const size_t frame_size = StringOutputBuffer::kZstdFrameSize;
  for (size_t data_size :
       {size_t(0), size_t(100), frame_size, 2 * frame_size + 300000}) {
    std::string data = CreateTestString(data_size);
    StringOutputBuffer buffer;
    // Several appends spread the data over several pages.
    for (size_t pos = 0; pos < data.size(); pos += 1000) {
      buffer.Append(std::string_view(data).substr(pos, 1000));
    }

    std::string compressed = buffer.CompressZstd().str();
    std::vector<size_t> frame_sizes;
    for (size_t pos = 0; pos < compressed.size();) {
      const char* frame = compressed.data() + pos;
      size_t n = ZSTD_findFrameCompressedSize(frame, compressed.size() - pos);
      ASSERT_FALSE(ZSTD_isError(n)) << ZSTD_getErrorName(n);
      frame_sizes.push_back(ZSTD_getFrameContentSize(frame, n));
      pos += n;
    }
    std::vector<size_t> want_sizes;
    for (size_t left = data_size;; left -= frame_size) {
      want_sizes.push_back(std::min(left, frame_size));
      if (left <= frame_size)
        break;
    }
    EXPECT_EQ(frame_sizes, want_sizes) << data_size;

    std::string decompressed(data_size, '\0');
    size_t n = ZSTD_decompress(decompressed.data(), decompressed.size(),
                               compressed.data(), compressed.size());
    ASSERT_FALSE(ZSTD_isError(n)) << ZSTD_getErrorName(n);
    EXPECT_EQ(n, data_size);
    EXPECT_EQ(decompressed, data);
  }
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