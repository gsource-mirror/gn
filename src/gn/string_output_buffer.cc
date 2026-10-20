// Copyright 2020 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/string_output_buffer.h"

#include <algorithm>
#include <array>
#include <cstring>
#include <fstream>

#include "base/compiler_specific.h"
#include "base/files/file_path.h"
#include "base/files/file_util.h"
#include "gn/err.h"
#include "gn/file_writer.h"
#include "gn/filesystem_utils.h"

void StringOutputBuffer::Reserve(size_t n) {
  if (UNLIKELY(page_free_size() < n)) {
    if (rw_.size > 0) {
      ro_.push_back(std::move(rw_));
      ro_size_ += rw_.size;
    }

    // Pick the minimum number of pages required to store n bytes.
    // So technically, our "Page" could actually be multiple pages
    // But unless you attempt to call append on a ridiculously large string,
    // this should never happen in practice.
    rw_capacity_ = (n + kPageSize - 1) & ~(kPageSize - 1);
    rw_ = Page{
        .buf = std::make_unique<char[]>(rw_capacity_),
        .size = 0,
    };
  }
}

std::string StringOutputBuffer::str() const {
  std::string result;
  result.reserve(size());
  for (const auto& page : ro_) {
    result.append(page.buf.get(), page.size);
  }
  if (rw_.size > 0) {
    result.append(rw_.buf.get(), rw_.size);
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

  auto check_page = [&](const Page& page) {
    size_t remaining = page.size;
    const char* page_ptr = page.buf.get();
    std::array<char, kPageSize> file_page;
    while (remaining > 0) {
      size_t chunk = std::min(remaining, kPageSize);
      file.read(file_page.data(), chunk);
      if (!file.good())
        return false;

      if (memcmp(file_page.data(), page_ptr, chunk) != 0)
        return false;

      page_ptr += chunk;
      remaining -= chunk;
    }
    return true;
  };

  for (const auto& page : ro_) {
    if (!check_page(page))
      return false;
  }
  if (rw_.size > 0) {
    if (!check_page(rw_))
      return false;
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
    auto write_page = [&](const Page& page) {
      if (page.size == 0)
        return true;
      return writer.Write(std::string_view(page.buf.get(), page.size));
    };

    for (const auto& page : ro_) {
      if (!write_page(page)) {
        success = false;
        break;
      }
    }
    if (success && rw_.size > 0) {
      if (!write_page(rw_)) {
        success = false;
      }
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
