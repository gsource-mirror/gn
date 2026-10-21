// Copyright (c) 2013 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "gn/escape.h"

#include <stddef.h>

#include <array>
#include <memory>

#include "base/compiler_specific.h"
#include "base/json/string_escape.h"
#include "base/logging.h"
#include "gn/string_output_buffer.h"
#include "util/build_config.h"

namespace {

#if defined(OS_WIN)
constexpr size_t kMaxEscapedCharsPerChar = 2;
#else
constexpr size_t kMaxEscapedCharsPerChar = 3;
#endif

// A "1" in this lookup table means that char is valid in the Posix shell.
// clang-format off
constexpr char kShellValid[256] = {
// 00-1f: all are invalid
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
// ' ' !  "  #  $  %  &  '  (  )  *  +  ,  -  .  /
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1,
//  0  1  2  3  4  5  6  7  8  9  :  ;  <  =  >  ?
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0,
//  @  A  B  C  D  E  F  G  H  I  J  K  L  M  N  O
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//  P  Q  R  S  T  U  V  W  X  Y  Z  [  \  ]  ^  _
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1,
//  `  a  b  c  d  e  f  g  h  i  j  k  l  m  n  o
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//  p  q  r  s  t  u  v  w  x  y  z  {  |  }  ~
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0};
// clang-format on

constexpr std::array<char, 256> PosixEscapesTo(const char (&entries)[256]) {
  std::array<char, 256> table{};
  for (size_t i = 0; i < 256; ++i) {
    if (i == ':') {
      table[i] = '$';
    } else if (entries[i] == 0) {
      table[i] = '\\';
    } else {
      table[i] = '\0';
    }
  }
  return table;
}

constexpr auto kPosixNinjaTable = PosixEscapesTo(kShellValid);

void EscapeStringToString_Space(std::string_view str,
                                char* dest,
                                size_t& upto) {
  size_t in_idx = 0;
  for (size_t delim = str.find(' '); UNLIKELY(delim != std::string_view::npos);
       delim = str.find(' ', in_idx)) {
    size_t len = delim - in_idx;
    memcpy(dest + upto, str.data() + in_idx, len);
    upto += len;
    dest[upto++] = '\\';
    dest[upto++] = ' ';
    in_idx = delim + 1;
  }
  size_t len = str.size() - in_idx;
  memcpy(dest + upto, str.data() + in_idx, len);
  upto += len;
}

// Ninja's escaping rules are very simple. We always escape colons even
// though they're OK in many places, in case the resulting string is used on
// the left-hand-side of a rule.
inline bool ShouldEscapeCharForNinja(char ch) {
  // Keep in sync with EscapeStringToString_Ninja
  return ch == '$' || ch == ' ' || ch == ':';
}

void EscapeStringToString_Ninja(std::string_view str,
                                char* dest,
                                size_t& upto) {
  size_t in_idx = 0;
  for (size_t delim = str.find_first_of("$ :");
       UNLIKELY(delim != std::string_view::npos);
       // Keep in sync with ShouldEscapeCharForNinja
       delim = str.find_first_of("$ :", in_idx)) {
    size_t len = delim - in_idx;
    memcpy(dest + upto, str.data() + in_idx, len);
    upto += len;
    dest[upto++] = '$';
    dest[upto++] = str[delim];
    in_idx = delim + 1;
  }
  size_t len = str.size() - in_idx;
  memcpy(dest + upto, str.data() + in_idx, len);
  upto += len;
}

inline bool ShouldEscapeCharForCompilationDatabase(char ch) {
  return ch == '\\' || ch == '"';
}

size_t EscapeStringToString_CompilationDatabase(std::string_view str,
                                                char* dest) {
  size_t i = 0;
  bool quote = false;
  for (const auto& elem : str) {
    if (!kShellValid[static_cast<int>(elem)]) {
      quote = true;
      break;
    }
  }
  if (quote)
    dest[i++] = '"';

  for (const auto& elem : str) {
    if (ShouldEscapeCharForCompilationDatabase(elem))
      dest[i++] = '\\';
    dest[i++] = elem;
  }
  if (quote)
    dest[i++] = '"';
  return i;
}

size_t EscapeStringToString_Depfile(std::string_view str, char* dest) {
  size_t i = 0;
  for (const auto& elem : str) {
    // Escape all characters that ninja depfile parser can recognize as escaped,
    // even if some of them can work without escaping.
    if (elem == ' ' || elem == '\\' || elem == '#' || elem == '*' ||
        elem == '[' || elem == '|' || elem == ']')
      dest[i++] = '\\';
    else if (elem == '$')  // Extra rule for $$
      dest[i++] = '$';
    dest[i++] = elem;
  }
  return i;
}

size_t EscapeStringToString_NinjaPreformatted(std::string_view str,
                                              char* dest) {
  // Only Ninja-escape $.
  size_t i = 0;
  for (const auto& elem : str) {
    if (elem == '$')
      dest[i++] = '$';
    dest[i++] = elem;
  }
  return i;
}

// Escape for CommandLineToArgvW and additionally escape Ninja characters.
//
// The basic algorithm is if the string doesn't contain any parse-affecting
// characters, don't do anything (other than the Ninja processing). If it does,
// quote the string, and backslash-escape all quotes and backslashes.
// See:
//   http://blogs.msdn.com/b/twistylittlepassagesallalike/archive/2011/04/23/everyone-quotes-arguments-the-wrong-way.aspx
//   http://blogs.msdn.com/b/oldnewthing/archive/2010/09/17/10063629.aspx
size_t EscapeStringToString_WindowsNinjaFork(std::string_view str,
                                             bool inhibit_quoting,
                                             char* dest,
                                             bool* needed_quoting) {
  // We assume we don't have any whitespace chars that aren't spaces.
  DCHECK(str.find_first_of("\r\n\v\t") == std::string::npos);

  size_t i = 0;
  // '$', ' ', and ':' mean we need ninja escaping. ' ' and '"' mean we need
  // shell escaping.
  size_t idx = str.find_first_of("$: \"");
  if (LIKELY(idx == std::string::npos)) {
    // Super-fast-path: No shell escaping or Ninja escaping required.
    memcpy(dest, str.data(), str.size());
    return str.size();
  }

  // If the special character found was NOT a space or quote (i.e. it was '$'
  // or ':'), check if there are any spaces or quotes in the rest of the string.
  if (str[idx] != ' ' && str[idx] != '"' &&
      str.find_first_of(" \"", idx + 1) == std::string::npos) {
    // Fast-path: Needs Ninja escaping, but no Windows command-line quoting.
    // We know that the up until here doesn't require any escaping
    memcpy(dest, str.data(), idx);
    size_t upto = idx;
    // Ninja always escapes with a $
    dest[upto++] = '$';
    dest[upto++] = str[idx];
    // Escape the rest of the string with ninja.
    EscapeStringToString_Ninja(str.substr(idx + 1), dest, upto);
    return upto;
  }

  if (!inhibit_quoting)
    dest[i++] = '"';

  for (size_t j = 0; j < str.size(); j++) {
    // Count backslashes in case they're followed by a quote.
    size_t backslash_count = 0;
    while (j < str.size() && str[j] == '\\') {
      j++;
      backslash_count++;
    }
    if (j == str.size()) {
      // Backslashes at end of string. Backslash-escape all of them since
      // they'll be followed by a quote.
      memset(dest + i, '\\', backslash_count * 2);
      i += backslash_count * 2;
    } else if (str[j] == '"') {
      // 0 or more backslashes followed by a quote. Backslash-escape the
      // backslashes, then backslash-escape the quote.
      memset(dest + i, '\\', backslash_count * 2 + 1);
      i += backslash_count * 2 + 1;
      dest[i++] = '"';
    } else {
      // Non-special Windows character, just escape for Ninja. Also, add any
      // backslashes we read previously, these are literals.
      memset(dest + i, '\\', backslash_count);
      i += backslash_count;
      if (ShouldEscapeCharForNinja(str[j]))
        dest[i++] = '$';
      dest[i++] = str[j];
    }
  }

  if (!inhibit_quoting)
    dest[i++] = '"';
  if (needed_quoting)
    *needed_quoting = true;

  return i;
}

void EscapeStringToString_PosixNinjaFork(std::string_view str,
                                         char* dest,
                                         size_t& upto) {
  auto find_first_escape = [&](size_t pos = 0) {
    for (size_t i = pos; i < str.size(); ++i) {
      if (kPosixNinjaTable[static_cast<uint8_t>(str[i])] != '\0')
        return i;
    }
    return std::string_view::npos;
  };

  size_t start = 0;
  for (size_t delim = find_first_escape();
       UNLIKELY(delim != std::string_view::npos);
       delim = find_first_escape(start)) {
    size_t len = delim - start;
    memcpy(dest + upto, str.data() + start, len);
    upto += len;

    char c = str[delim];
    dest[upto++] = kPosixNinjaTable[static_cast<uint8_t>(c)];
    if (c == ' ' || c == '$') {
      dest[upto++] = '$';
    }
    dest[upto++] = c;
    start = delim + 1;
  }
  size_t len = str.size() - start;
  memcpy(dest + upto, str.data() + start, len);
  upto += len;
}

// Escapes |str| into |dest| and returns the number of characters written.
size_t EscapeStringToString(std::string_view str,
                            const EscapeOptions& options,
                            char* dest,
                            bool* needed_quoting) {
  switch (options.mode) {
    case ESCAPE_NONE:
      strncpy(dest, str.data(), str.size());
      return str.size();
    case ESCAPE_SPACE: {
      size_t upto = 0;
      EscapeStringToString_Space(str, dest, upto);
      return upto;
    }
    case ESCAPE_NINJA: {
      size_t upto = 0;
      EscapeStringToString_Ninja(str, dest, upto);
      return upto;
    }
    case ESCAPE_DEPFILE:
      return EscapeStringToString_Depfile(str, dest);
    case ESCAPE_COMPILATION_DATABASE:
      return EscapeStringToString_CompilationDatabase(str, dest);
    case ESCAPE_NINJA_COMMAND:
      switch (options.platform) {
        case ESCAPE_PLATFORM_CURRENT:
#if defined(OS_WIN)
          return EscapeStringToString_WindowsNinjaFork(
              str, options.inhibit_quoting, dest, needed_quoting);
#else
        {
          size_t upto = 0;
          EscapeStringToString_PosixNinjaFork(str, dest, upto);
          return upto;
        }
#endif
        case ESCAPE_PLATFORM_WIN:
          return EscapeStringToString_WindowsNinjaFork(
              str, options.inhibit_quoting, dest, needed_quoting);
        case ESCAPE_PLATFORM_POSIX: {
          size_t upto = 0;
          EscapeStringToString_PosixNinjaFork(str, dest, upto);
          return upto;
        }
        default:
          NOTREACHED();
      }
    case ESCAPE_NINJA_PREFORMATTED_COMMAND:
      return EscapeStringToString_NinjaPreformatted(str, dest);
    default:
      NOTREACHED();
  }
  return 0;
}

}  // namespace

void Escape::Space::WriteToBuf(char buf[], size_t& upto) const {
  EscapeStringToString_Space(str, buf, upto);
}

void Escape::Ninja::WriteToBuf(char buf[], size_t& upto) const {
  EscapeStringToString_Ninja(str, buf, upto);
}

void Escape::Depfile::WriteToBuf(char buf[], size_t& upto) const {
  upto += EscapeStringToString_Depfile(str, &buf[upto]);
}

void Escape::CompilationDatabase::WriteToBuf(char buf[], size_t& upto) const {
  upto += EscapeStringToString_CompilationDatabase(str, &buf[upto]);
}

void Escape::NinjaPreformatted::WriteToBuf(char buf[], size_t& upto) const {
  upto += EscapeStringToString_NinjaPreformatted(str, &buf[upto]);
}

void Escape::Command::WriteToBuf(char buf[], size_t& upto) const {
  switch (platform) {
    case ESCAPE_PLATFORM_CURRENT:
#if defined(OS_WIN)
      upto += EscapeStringToString_WindowsNinjaFork(str, inhibit_quoting,
                                                    &buf[upto], nullptr);
#else
      EscapeStringToString_PosixNinjaFork(str, buf, upto);
#endif
      break;
    case ESCAPE_PLATFORM_WIN:
      upto += EscapeStringToString_WindowsNinjaFork(str, inhibit_quoting,
                                                    &buf[upto], nullptr);
      break;
    case ESCAPE_PLATFORM_POSIX:
      EscapeStringToString_PosixNinjaFork(str, buf, upto);
      break;
  }
}

std::string EscapeString(std::string_view str,
                         const EscapeOptions& options,
                         bool* needed_quoting) {
  std::string result;
  result.resize(str.size() * kMaxEscapedCharsPerChar + 2);
  size_t len =
      EscapeStringToString(str, options, result.data(), needed_quoting);
  result.resize(len);
  return result;
}

void EscapeStringToStream(StringOutputBuffer& out,
                          std::string_view str,
                          const EscapeOptions& options) {
  switch (options.mode) {
    case ESCAPE_NONE:
      out << str;
      break;
    case ESCAPE_SPACE:
      out << Escape::Space(str);
      break;
    case ESCAPE_NINJA:
      out << Escape::Ninja(str);
      break;
    case ESCAPE_DEPFILE:
      out << Escape::Depfile(str);
      break;
    case ESCAPE_COMPILATION_DATABASE:
      out << Escape::CompilationDatabase(str);
      break;
    case ESCAPE_NINJA_COMMAND:
      out << Escape::Command(str, options.platform, options.inhibit_quoting);
      break;
    case ESCAPE_NINJA_PREFORMATTED_COMMAND:
      out << Escape::NinjaPreformatted(str);
      break;
    default:
      NOTREACHED();
  }
}

void EscapeJSONStringToStream(StringOutputBuffer& out,
                              std::string_view str,
                              const EscapeOptions& options) {
  std::string dest;
  bool needed_quoting = !options.inhibit_quoting;
  base::EscapeJSONString(str, needed_quoting, &dest);

  EscapeStringToStream(out, dest, options);
}
