#pragma once
#include <cstddef>
#include <cstdint>
#include <memory>
#include <string>
#include <type_traits>
#include "gn/ffi/test_with_scope.h"
#include "gn/label.h"
#include "gn/output_file.h"
#include "gn/scope.h"
#include "gn/settings.h"
#include "gn/source_dir.h"
#include "gn/test_with_scope.h"

#if __cplusplus >= 201402L
#define CXX_DEFAULT_VALUE(value) = value
#else
#define CXX_DEFAULT_VALUE(value)
#endif

struct RustFfiStringView;
using OutputFile = ::OutputFile;
using SourceDir = ::SourceDir;
using Label = ::Label;
using Settings = ::Settings;
using Scope = ::Scope;
using TestWithScope = ::TestWithScope;

#ifndef CXXBRIDGE1_STRUCT_RustFfiStringView
#define CXXBRIDGE1_STRUCT_RustFfiStringView
// This is a copy of the StringView type defined in string_view.rs.
// We need a type for C++ to include in the FFI layout test, and the real
// one we told rust to map to std::string_view.
struct RustFfiStringView final {
  ::std::size_t len CXX_DEFAULT_VALUE(0);
  ::std::uint8_t const* ptr CXX_DEFAULT_VALUE(nullptr);

  using IsRelocatable = ::std::true_type;
};
#endif  // CXXBRIDGE1_STRUCT_RustFfiStringView
