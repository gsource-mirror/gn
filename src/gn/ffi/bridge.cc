#include <cstddef>
#include <cstdint>
#include <memory>
#include <new>
#include <string>
#include <type_traits>
#include <utility>
#include "gn/ffi/test_with_scope.h"
#include "gn/label.h"
#include "gn/output_file.h"
#include "gn/scope.h"
#include "gn/settings.h"
#include "gn/source_dir.h"
#include "gn/test_with_scope.h"

#ifdef __GNUC__
#pragma GCC diagnostic ignored "-Wmissing-declarations"
#ifdef __clang__
#pragma clang diagnostic ignored "-Wdollar-in-identifier-extension"
#endif  // __clang__
#endif  // __GNUC__

namespace rust {
inline namespace cxxbridge1 {
// #include "rust/cxx.h"

#ifndef CXXBRIDGE1_IS_COMPLETE
#define CXXBRIDGE1_IS_COMPLETE
namespace detail {
namespace {
template <typename T, typename = std::size_t>
struct is_complete : std::false_type {};
template <typename T>
struct is_complete<T, decltype(sizeof(T))> : std::true_type {};
}  // namespace
}  // namespace detail
#endif  // CXXBRIDGE1_IS_COMPLETE

#ifndef CXXBRIDGE1_RELOCATABLE
#define CXXBRIDGE1_RELOCATABLE
namespace detail {
template <typename... Ts>
struct make_void {
  using type = void;
};

template <typename... Ts>
using void_t = typename make_void<Ts...>::type;

template <typename Void, template <typename...> class, typename...>
struct detect : std::false_type {};
template <template <typename...> class T, typename... A>
struct detect<void_t<T<A...>>, T, A...> : std::true_type {};

template <template <typename...> class T, typename... A>
using is_detected = detect<void, T, A...>;

template <typename T>
using detect_IsRelocatable = typename T::IsRelocatable;

template <typename T>
struct get_IsRelocatable
    : std::is_same<typename T::IsRelocatable, std::true_type> {};
}  // namespace detail

template <typename T>
struct IsRelocatable
    : std::conditional<
          detail::is_detected<detail::detect_IsRelocatable, T>::value,
          detail::get_IsRelocatable<T>,
          std::integral_constant<
              bool,
              std::is_trivially_move_constructible<T>::value &&
                  std::is_trivially_destructible<T>::value>>::type {};
#endif  // CXXBRIDGE1_RELOCATABLE

namespace {
template <bool>
struct deleter_if {
  template <typename T>
  void operator()(T*) {}
};
template <>
struct deleter_if<true> {
  template <typename T>
  void operator()(T* ptr) {
    ptr->~T();
  }
};
}  // namespace
}  // namespace cxxbridge1
}  // namespace rust

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

static_assert(::rust::IsRelocatable<::std::string_view>::value,
              "type std::string_view should be trivially move constructible "
              "and trivially destructible in C++ to be used as a return value "
              "of `value`, `SourceWithNoTrailingSlash` in Rust");

extern "C" {
void cxxbridge1$194$OutputFile$value(::OutputFile const& self,
                                     ::std::string_view* return$) noexcept {
  ::std::string_view (::OutputFile::*value$)() const = &::OutputFile::value;
  new (return$)::std::string_view((self.*value$)());
}

void cxxbridge1$194$SourceDir$SourceWithNoTrailingSlash(
    ::SourceDir const& self,
    ::std::string_view* return$) noexcept {
  ::std::string_view (::SourceDir::*SourceWithNoTrailingSlash$)() const =
      &::SourceDir::SourceWithNoTrailingSlash;
  new (return$)::std::string_view((self.*SourceWithNoTrailingSlash$)());
}

void cxxbridge1$194$Label$dir(::Label const& self,
                              ::SourceDir const** return$) noexcept {
  ::SourceDir const& (::Label::*dir$)() const = &::Label::dir;
  new (return$)::SourceDir const*(&(self.*dir$)());
}

void cxxbridge1$194$Label$name_cxx(::Label const& self,
                                   ::std::string const** return$) noexcept {
  ::std::string const& (::Label::*name_cxx$)() const = &::Label::name;
  new (return$)::std::string const*(&(self.*name_cxx$)());
}

void cxxbridge1$194$Settings$toolchain_label(::Settings const& self,
                                             ::Label const** return$) noexcept {
  ::Label const& (::Settings::*toolchain_label$)() const =
      &::Settings::toolchain_label;
  new (return$)::Label const*(&(self.*toolchain_label$)());
}

::Settings const* cxxbridge1$194$Scope$settings_cxx(
    ::Scope const& self) noexcept {
  ::Settings const* (::Scope::*settings_cxx$)() const = &::Scope::settings;
  return (self.*settings_cxx$)();
}

::TestWithScope* cxxbridge1$194$NewTestWithScope() noexcept {
  ::std::unique_ptr<::TestWithScope> (*NewTestWithScope$)() =
      ::NewTestWithScope;
  return NewTestWithScope$().release();
}

::Scope* cxxbridge1$194$TestWithScope$scope_cxx(
    ::TestWithScope& self) noexcept {
  ::Scope* (::TestWithScope::*scope_cxx$)() = &::TestWithScope::scope;
  return (self.*scope_cxx$)();
}

static_assert(::rust::detail::is_complete<
                  ::std::remove_extent<::TestWithScope>::type>::value,
              "definition of `::TestWithScope` is required");
static_assert(sizeof(::std::unique_ptr<::TestWithScope>) == sizeof(void*), "");
static_assert(alignof(::std::unique_ptr<::TestWithScope>) == alignof(void*),
              "");
void cxxbridge1$unique_ptr$TestWithScope$null(
    ::std::unique_ptr<::TestWithScope>* ptr) noexcept {
  ::new (ptr)::std::unique_ptr<::TestWithScope>();
}
void cxxbridge1$unique_ptr$TestWithScope$raw(
    ::std::unique_ptr<::TestWithScope>* ptr,
    ::std::unique_ptr<::TestWithScope>::pointer raw) noexcept {
  ::new (ptr)::std::unique_ptr<::TestWithScope>(raw);
}
::std::unique_ptr<::TestWithScope>::element_type const*
cxxbridge1$unique_ptr$TestWithScope$get(
    ::std::unique_ptr<::TestWithScope> const& ptr) noexcept {
  return ptr.get();
}
::std::unique_ptr<::TestWithScope>::pointer
cxxbridge1$unique_ptr$TestWithScope$release(
    ::std::unique_ptr<::TestWithScope>& ptr) noexcept {
  return ptr.release();
}
void cxxbridge1$unique_ptr$TestWithScope$drop(
    ::std::unique_ptr<::TestWithScope>* ptr) noexcept {
  ::rust::deleter_if<::rust::detail::is_complete<::TestWithScope>::value>{}(
      ptr);
}
}  // extern "C"
