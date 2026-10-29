// Copyright 2020 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/string_output_buffer.h"

#include <algorithm>
#include <fstream>

#include "base/compiler_specific.h"
#include "base/files/file_path.h"
#include "base/files/file_util.h"
#include "base/logging.h"
#include "gn/err.h"
#include "gn/file_writer.h"
#include "gn/filesystem_utils.h"
#include "third_party/zstd/lib/zstd.h"

namespace {

// Random access to content that is split into pages.
class PagedContent {
 public:
  void AddPage(std::string_view page) {
    if (page.empty())
      return;
    starts_.push_back(size_);
    pages_.push_back(page);
    size_ += page.size();
  }

  size_t size() const { return size_; }

  char operator[](size_t i) const {
    size_t k = PageIndex(i);
    return pages_[k][i - starts_[k]];
  }

  // Returns the offset of the first '\n' at or after |i|, or size().
  size_t FindNewline(size_t i) const {
    for (size_t k = PageIndex(i); k < pages_.size(); ++k) {
      size_t pos = pages_[k].find('\n', std::max(i, starts_[k]) - starts_[k]);
      if (pos != std::string_view::npos)
        return starts_[k] + pos;
    }
    return size_;
  }

 private:
  size_t PageIndex(size_t i) const {
    return std::upper_bound(starts_.begin(), starts_.end(), i) -
           starts_.begin() - 1;
  }

  std::vector<std::string_view> pages_;
  std::vector<size_t> starts_;
  size_t size_ = 0;
};

// Returns true if a top-level Ninja statement starts at |i|: it follows a line
// that isn't continued with "$", and it isn't an indented binding, a comment
// or an empty line. This is Siso's chunk boundary rule, so Siso can parse
// frames that start at such offsets independently.
bool IsStatementStart(const PagedContent& content, size_t i) {
  if (i == 0 || i >= content.size() || content[i - 1] != '\n')
    return false;
  if (i >= 2 && content[i - 2] == '$')
    return false;
  if (i >= 3 && content[i - 3] == '$' && content[i - 2] == '\r')
    return false;
  switch (content[i]) {
    case ' ':
    case '\t':
    case '#':
    case '\r':
    case '\n':
      return false;
  }
  return true;
}

// Returns the first statement start after |i|, or content.size().
size_t NextStatementStart(const PagedContent& content, size_t i) {
  while (true) {
    i = content.FindNewline(i);
    if (i >= content.size())
      return content.size();
    ++i;
    if (IsStatementStart(content, i))
      return i;
  }
}

}  // namespace

void StringOutputBuffer::Clear() {
  ro_.clear();
  cur_ = rw_buf_.get();
}

void StringOutputBuffer::ReserveSlow(size_t n) {
  if (rw_buf_) {
    size_t page_size = cur_ - rw_buf_.get();
    if (page_size > 0) {
      ro_.push_back(Page{
          .buf = std::move(rw_buf_),
          .size = page_size,
      });
    }
  }

  // Pick the minimum number of pages required to store n bytes.
  // So technically, our "Page" could actually be multiple pages
  // But unless you attempt to call append on a ridiculously large string,
  // this should never happen in practice.
  size_t capacity = (n + kPageSize - 1) & ~(kPageSize - 1);
  rw_buf_ = std::make_unique_for_overwrite<char[]>(capacity);
  cur_ = rw_buf_.get();
  end_ = cur_ + capacity;
}

size_t StringOutputBuffer::size() const {
  size_t total = rw_buf_ ? static_cast<size_t>(cur_ - rw_buf_.get()) : 0;
  for (const auto& page : ro_) {
    total += page.size;
  }
  return total;
}

std::string StringOutputBuffer::str() const {
  std::string result;
  result.reserve(size());
  for (const auto& page : ro_) {
    result.append(page.buf.get(), page.size);
  }
  if (rw_buf_) {
    result.append(rw_buf_.get(), cur_);
  }
  return result;
}

StringOutputBuffer StringOutputBuffer::CompressZstd(size_t frame_size) const {
  std::unique_ptr<ZSTD_CCtx, decltype(&ZSTD_freeCCtx)> cctx(ZSTD_createCCtx(),
                                                            &ZSTD_freeCCtx);
  CHECK(cctx);
  CHECK(!ZSTD_isError(
      ZSTD_CCtx_setParameter(cctx.get(), ZSTD_c_checksumFlag, 1)));

  StringOutputBuffer result;
  const size_t chunk_size = ZSTD_CStreamOutSize();
  auto chunk = std::make_unique_for_overwrite<char[]>(chunk_size);
  auto compress = [&](const char* data, size_t size, ZSTD_EndDirective mode) {
    ZSTD_inBuffer input = {data, size, 0};
    size_t remaining;
    do {
      ZSTD_outBuffer output = {chunk.get(), chunk_size, 0};
      remaining = ZSTD_compressStream2(cctx.get(), &output, &input, mode);
      CHECK(!ZSTD_isError(remaining)) << ZSTD_getErrorName(remaining);
      result.Append(chunk.get(), output.pos);
    } while (mode == ZSTD_e_end ? remaining != 0 : input.pos < input.size);
  };

  // Each frame ends at the first statement start after |frame_size| bytes.
  PagedContent content;
  for (const auto& page : ro_) {
    content.AddPage(std::string_view(page.buf.get(), page.size));
  }
  if (rw_buf_) {
    content.AddPage(std::string_view(rw_buf_.get(), cur_ - rw_buf_.get()));
  }
  std::vector<size_t> frame_ends;
  for (size_t end = 0; end < content.size();) {
    end = end + frame_size >= content.size()
              ? content.size()
              : NextStatementStart(content, end + frame_size - 1);
    frame_ends.push_back(end);
  }

  // Start a new frame at each of those offsets, declaring its size.
  size_t written = 0;
  size_t next_frame = 0;
  size_t frame_left = 0;
  auto start_frame = [&] {
    frame_left =
        next_frame < frame_ends.size() ? frame_ends[next_frame++] - written : 0;
    CHECK(!ZSTD_isError(ZSTD_CCtx_setPledgedSrcSize(cctx.get(), frame_left)));
  };
  auto append = [&](const char* data, size_t size) {
    while (size > 0) {
      if (frame_left == 0)
        start_frame();
      size_t n = std::min(size, frame_left);
      frame_left -= n;
      written += n;
      compress(data, n, frame_left == 0 ? ZSTD_e_end : ZSTD_e_continue);
      data += n;
      size -= n;
    }
  };
  for (const auto& page : ro_) {
    append(page.buf.get(), page.size);
  }
  if (rw_buf_) {
    append(rw_buf_.get(), cur_ - rw_buf_.get());
  }
  if (result.size() == 0) {
    // Empty content still takes one frame.
    start_frame();
    compress(nullptr, 0, ZSTD_e_end);
  }
  return result;
}

bool StringOutputBuffer::ContentsEqual(const base::FilePath& file_path) const {
  // Compare file and stream sizes first. Quick and will save us some time if
  // they are different sizes.
  int64_t file_size;
  if (!base::GetFileSize(file_path, &file_size) ||
      static_cast<size_t>(file_size) != size()) {
    return false;
  }

  // Open the file in binary mode.
  std::ifstream file(file_path.As8Bit().c_str(), std::ios::binary);
  if (!file.is_open())
    return false;

  std::vector<char> file_page;
  file_page.resize(kPageSize);
  auto check_page = [&](const char* page_buf, size_t page_size) {
    // Don't use reserve because if we do that, capacity is high enough but not
    // size.
    if (UNLIKELY(file_page.size() < page_size)) {
      file_page.resize(page_size);
    }
    file.read(file_page.data(), page_size);
    if (!file.good())
      return false;

    return memcmp(file_page.data(), page_buf, page_size) == 0;
  };

  for (const auto& page : ro_) {
    if (!check_page(page.buf.get(), page.size))
      return false;
  }
  if (cur_ > rw_buf_.get()) {
    return check_page(rw_buf_.get(), cur_ - rw_buf_.get());
  }
  return true;
}

// Write the contents of this instance to a file at |file_path|.
bool StringOutputBuffer::WriteToFile(const base::FilePath& file_path,
                                     Err* err) const {
  // Create the directory if necessary.
  if (!base::CreateDirectory(file_path.DirName())) {
    if (err) {
      *err =
          Err(Location(), "Unable to create directory.",
              "I was using \"" + FilePathToUTF8(file_path.DirName()) + "\".");
    }
    return false;
  }

  FileWriter writer;
  bool success = writer.Create(file_path);
  if (success) {
    for (const auto& page : ro_) {
      success = writer.Write(std::string_view(page.buf.get(), page.size));
      if (!success)
        break;
    }
    if (success && rw_buf_) {
      success = writer.Write(std::string_view(rw_buf_.get(), cur_));
    }
  }
  if (!writer.Close())
    success = false;

  if (!success && err) {
    *err = Err(Location(), "Unable to write file.",
               "I was writing \"" + FilePathToUTF8(file_path) + "\".");
  }
  return success;
}

bool StringOutputBuffer::WriteToFileIfChanged(const base::FilePath& file_path,
                                              Err* err) const {
  if (ContentsEqual(file_path))
    return true;

  return WriteToFile(file_path, err);
}

std::string Pretty(const StringOutputBuffer& buffer) {
  return buffer.str();
}
