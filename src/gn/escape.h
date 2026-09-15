// Copyright (c) 2013 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TOOLS_GN_ESCAPE_H_
#define TOOLS_GN_ESCAPE_H_

#include <cstring>
#include <ostream>
#include <string>
#include <string_view>

#if defined(__x86_64__) || defined(_M_X64) || defined(__i386__) || \
    defined(_M_IX86)
#include <emmintrin.h>
#endif

enum EscapingMode {
  // No escaping.
  ESCAPE_NONE,

  // Space only.
  ESCAPE_SPACE,

  // Ninja string escaping.
  ESCAPE_NINJA,

  // Ninja/makefile depfile string escaping.
  ESCAPE_DEPFILE,

  // For writing commands to ninja files. This assumes the output is "one
  // thing" like a filename, so will escape or quote spaces as necessary for
  // both Ninja and the shell to keep that thing together.
  ESCAPE_NINJA_COMMAND,

  // For writing preformatted shell commands to Ninja files. This assumes the
  // output already has the proper quoting and may include special shell
  // characters which we want to pass to the shell (like when writing tool
  // commands). Only Ninja "$" are escaped.
  ESCAPE_NINJA_PREFORMATTED_COMMAND,

  // Shell escaping as described by JSON Compilation Database spec:
  // Parameters use shell quoting and shell escaping of quotes, with ‘"’ and ‘\’
  // being the only special characters.
  ESCAPE_COMPILATION_DATABASE,
};

enum EscapingPlatform {
  // Do escaping for the current platform.
  ESCAPE_PLATFORM_CURRENT,

  // Force escaping for the given platform.
  ESCAPE_PLATFORM_POSIX,
  ESCAPE_PLATFORM_WIN,
};

struct EscapeOptions {
  EscapingMode mode = ESCAPE_NONE;

  // Controls how "fork" escaping is done. You will generally want to keep the
  // default "current" platform.
  EscapingPlatform platform = ESCAPE_PLATFORM_CURRENT;

  // When the escaping mode is ESCAPE_SHELL, the escaper will normally put
  // quotes around things with spaces. If this value is set to true, we'll
  // disable the quoting feature and just add the spaces.
  //
  // This mode is for when quoting is done at some higher-level. Defaults to
  // false. Note that Windows has strange behavior where the meaning of the
  // backslashes changes according to if it is followed by a quote. The
  // escaping rules assume that a double-quote will be appended to the result.
  bool inhibit_quoting = false;
};

namespace internal {

// Returns true if the string |str| needs any escaping for Ninja using a scalar
// loop.
inline bool NeedsEscapeNinjaScalar(std::string_view str) {
  for (char c : str) {
    if (c == '$' || c == ' ' || c == ':')
      return true;
  }
  return false;
}

#if defined(__x86_64__) || defined(_M_X64) || defined(__i386__) || \
    defined(_M_IX86)
// Returns true if the string |str| needs any escaping for Ninja using SSE2 SIMD
// instructions.
inline bool NeedsEscapeNinjaSIMD(std::string_view str) {
  const char* p = str.data();
  size_t len = str.size();
  const __m128i space = _mm_set1_epi8(' ');
  const __m128i dollar = _mm_set1_epi8('$');
  const __m128i colon = _mm_set1_epi8(':');
  if (len >= 16) {
    const char* end = p + len;
    while (p + 16 <= end) {
      __m128i chunk = _mm_loadu_si128(reinterpret_cast<const __m128i*>(p));
      __m128i m = _mm_or_si128(_mm_or_si128(_mm_cmpeq_epi8(chunk, space),
                                            _mm_cmpeq_epi8(chunk, dollar)),
                               _mm_cmpeq_epi8(chunk, colon));
      if (_mm_movemask_epi8(m) != 0)
        return true;
      p += 16;
    }
    if (p < end) {
      __m128i chunk =
          _mm_loadu_si128(reinterpret_cast<const __m128i*>(end - 16));
      __m128i m = _mm_or_si128(_mm_or_si128(_mm_cmpeq_epi8(chunk, space),
                                            _mm_cmpeq_epi8(chunk, dollar)),
                               _mm_cmpeq_epi8(chunk, colon));
      if (_mm_movemask_epi8(m) != 0)
        return true;
    }
    return false;
  }
#if defined(__x86_64__) || defined(_M_X64)
  if (len >= 8) {
    uint64_t lo, hi;
    memcpy(&lo, p, 8);
    memcpy(&hi, p + len - 8, 8);
    __m128i chunk = _mm_set_epi64x(hi, lo);
    __m128i m = _mm_or_si128(_mm_or_si128(_mm_cmpeq_epi8(chunk, space),
                                          _mm_cmpeq_epi8(chunk, dollar)),
                             _mm_cmpeq_epi8(chunk, colon));
    return _mm_movemask_epi8(m) != 0;
  }
#endif
  if (len >= 4) {
    uint32_t lo, hi;
    memcpy(&lo, p, 4);
    memcpy(&hi, p + len - 4, 4);
    __m128i chunk = _mm_set_epi32(0, 0, hi, lo);
    __m128i m = _mm_or_si128(_mm_or_si128(_mm_cmpeq_epi8(chunk, space),
                                          _mm_cmpeq_epi8(chunk, dollar)),
                             _mm_cmpeq_epi8(chunk, colon));
    return _mm_movemask_epi8(m) != 0;
  }
  for (size_t i = 0; i < len; ++i) {
    char c = p[i];
    if (c == '$' || c == ' ' || c == ':')
      return true;
  }
  return false;
}
#endif

bool NeedsEscapeSlow(std::string_view str, const EscapeOptions& options);

// Returns true if the string |str| needs any escaping for the given |options|.
inline bool NeedsEscape(std::string_view str, const EscapeOptions& options) {
  if (options.mode == ESCAPE_NINJA) {
#if defined(__x86_64__) || defined(_M_X64) || defined(__i386__) || \
    defined(_M_IX86)
    return NeedsEscapeNinjaSIMD(str);
#else
    return NeedsEscapeNinjaScalar(str);
#endif
  }
  if (options.mode == ESCAPE_NONE)
    return false;
  return NeedsEscapeSlow(str, options);
}

}  // namespace internal

// Escapes the given input, returnining the result.
//
// If needed_quoting is non-null, whether the string was or should have been
// (if inhibit_quoting was set) quoted will be written to it. This value should
// be initialized to false by the caller and will be written to only if it's
// true (the common use-case is for chaining calls).
std::string EscapeString(std::string_view str,
                         const EscapeOptions& options,
                         bool* needed_quoting);

void EscapeStringToStreamSlow(std::ostream& out,
                              std::string_view str,
                              const EscapeOptions& options);

namespace internal {

struct StreambufFastWriter : public std::streambuf {
  inline void WriteString(std::string_view str) {
    size_t len = str.size();
    if (static_cast<size_t>(epptr() - pptr()) >= len) {
      if (len > 0) {
        memcpy(pptr(), str.data(), len);
        pbump(static_cast<int>(len));
      }
    } else {
      sputn(str.data(), static_cast<std::streamsize>(len));
    }
  }

  inline void WriteCharAndString(char c, std::string_view str) {
    size_t len = str.size();
    if (static_cast<size_t>(epptr() - pptr()) >= len + 1) {
      char* p = pptr();
      *p = c;
      memcpy(p + 1, str.data(), len);
      pbump(static_cast<int>(len + 1));
    } else {
      sputc(c);
      sputn(str.data(), static_cast<std::streamsize>(len));
    }
  }
};

}  // namespace internal

// Same as EscapeString but writes the results to the given stream, saving a
// copy.
inline void EscapeStringToStream(std::ostream& out,
                                 std::string_view str,
                                 const EscapeOptions& options) {
  if (!internal::NeedsEscape(str, options)) {
    static_cast<internal::StreambufFastWriter*>(out.rdbuf())->WriteString(str);
    return;
  }
  EscapeStringToStreamSlow(out, str, options);
}

// Same as EscapeString but escape JSON string and writes the results to the
// given stream, saving a copy.
void EscapeJSONStringToStream(std::ostream& out,
                              std::string_view str,
                              const EscapeOptions& options);

#endif  // TOOLS_GN_ESCAPE_H_
