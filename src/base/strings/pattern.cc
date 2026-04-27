// Copyright 2026 The Chromium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "base/strings/pattern.h"

#include <string_view>

#include "base/compiler_specific.h"
#include "base/third_party/icu/icu_utf.h"

namespace base {

namespace {

constexpr bool IsWildcard(base_icu::UChar32 character) {
  return character == '*' || character == '?';
}

template <typename CHAR, typename NEXT>
constexpr bool SearchForChars(const CHAR** pattern,
                              const CHAR* pattern_end,
                              const CHAR** string,
                              const CHAR* string_end,
                              int maximum_distance,
                              NEXT next) {
  const CHAR* pattern_start = *pattern;
  const CHAR* string_start = *string;
  bool escape = false;
  while (true) {
    if (*pattern == pattern_end) {
      if (*string == string_end) {
        return true;
      }
    } else {
      if (!escape && IsWildcard(**pattern)) {
        return true;
      }

      if (!escape && **pattern == '\\') {
        escape = true;
        next(pattern, pattern_end);
        continue;
      }

      escape = false;

      if (*string == string_end) {
        return false;
      }

      const CHAR* pattern_next = *pattern;
      const CHAR* string_next = *string;
      base_icu::UChar32 pattern_char = next(&pattern_next, pattern_end);
      if (pattern_char == next(&string_next, string_end) &&
          pattern_char != CBU_SENTINEL) {
        *pattern = pattern_next;
        *string = string_next;
        continue;
      }
    }

    if (maximum_distance == 0) {
      return false;
    }

    maximum_distance--;
    *pattern = pattern_start;
    next(&string_start, string_end);
    *string = string_start;
  }
}

template <typename CHAR, typename NEXT>
constexpr int EatWildcards(const CHAR** pattern, const CHAR* end, NEXT next) {
  int num_question_marks = 0;
  bool has_asterisk = false;
  while (*pattern != end) {
    if (**pattern == '?') {
      num_question_marks++;
    } else if (**pattern == '*') {
      has_asterisk = true;
    } else {
      break;
    }

    next(pattern, end);
  }
  return has_asterisk ? -1 : num_question_marks;
}

template <typename CHAR, typename NEXT>
constexpr bool MatchPatternT(const CHAR* eval,
                             const CHAR* eval_end,
                             const CHAR* pattern,
                             const CHAR* pattern_end,
                             NEXT next) {
  do {
    int maximum_wildcard_length = EatWildcards(&pattern, pattern_end, next);
    if (!SearchForChars(&pattern, pattern_end, &eval, eval_end,
                        maximum_wildcard_length, next)) {
      return false;
    }
  } while (pattern != pattern_end);
  return true;
}

struct NextCharUTF8 {
  base_icu::UChar32 operator()(const char** p, const char* end) {
    base_icu::UChar32 c;
    int offset = 0;
    CBU8_NEXT(reinterpret_cast<const uint8_t*>(*p), offset, end - *p, c);
    *p += offset;
    return c;
  }
};

struct NextCharUTF16 {
  base_icu::UChar32 operator()(const char16_t** p, const char16_t* end) {
    base_icu::UChar32 c;
    int offset = 0;
    CBU16_NEXT(*p, offset, end - *p, c);
    *p += offset;
    return c;
  }
};

}  // namespace

bool MatchPattern(std::string_view eval, std::string_view pattern) {
  return MatchPatternT(
      eval.data(), eval.data() + eval.size(), pattern.data(),
      pattern.data() + pattern.size(), NextCharUTF8());
}

bool MatchPattern(std::u16string_view eval, std::u16string_view pattern) {
  return MatchPatternT(
      eval.data(), eval.data() + eval.size(), pattern.data(),
      pattern.data() + pattern.size(), NextCharUTF16());
}

}  // namespace base
