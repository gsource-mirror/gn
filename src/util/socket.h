// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef UTIL_SOCKET_H_
#define UTIL_SOCKET_H_

#include <cstdint>
#include <memory>
#include <optional>
#include <string_view>
#include <tuple>
#include <vector>

#include "base/containers/span.h"

#if defined(OS_WIN)
#include <winsock2.h>
using SocketType = SOCKET;
#else
using SocketType = int;
#endif

namespace util {

template <typename T>
void SerializeData(const T* begin, size_t size, std::vector<uint8_t>& data) {
  std::copy(reinterpret_cast<const uint8_t*>(begin),
            reinterpret_cast<const uint8_t*>(begin + size),
            std::back_inserter(data));
}

template <typename T>
void SerializeLiteral(T value, std::vector<uint8_t>& data) {
  SerializeData(reinterpret_cast<const uint8_t*>(&value), sizeof(value), data);
}

void SerializeString(std::string_view s, std::vector<uint8_t>& data);

// The deserialize functions take a mutable span, and "consume" it by advancing
// the start pointer of it.

template <typename T>
T* DeserializeLiteral(base::span<uint8_t>& span) {
  if (span.size() < sizeof(T)) {
    return nullptr;
  }
  T* val = reinterpret_cast<T*>(span.data());
  span = span.subspan(sizeof(T));
  return val;
}

std::optional<std::string_view> DeserializeString(base::span<uint8_t>& span);

class Socket {
 public:
  Socket(SocketType fd);
  Socket(const Socket&) = delete;
  Socket& operator=(const Socket&) = delete;
  virtual ~Socket();

  std::optional<SocketType> fd() const;
  bool Send(uint32_t kind, base::span<uint8_t> payload) const;
  std::tuple<std::vector<uint8_t>, uint32_t, bool> Receive() const;
  static std::unique_ptr<Socket> Connect(int port);

 protected:
  SocketType fd_;
};

class ServerSocket : public Socket {
 public:
  ServerSocket(SocketType fd) : Socket(fd) {}

  std::unique_ptr<Socket> Accept();
  static std::unique_ptr<ServerSocket> Listen(int port);
};

}  // namespace util

#endif  // UTIL_SOCKET_H_
