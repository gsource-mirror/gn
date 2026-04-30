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
#include "util/build_config.h"

namespace util {

template <typename T>
void SerializeData(const T* begin, size_t size, std::vector<uint8_t>& data) {
  std::copy(reinterpret_cast<const uint8_t*>(begin),
            reinterpret_cast<const uint8_t*>(begin) + size,
            std::back_inserter(data));
}

template <typename T>
void SerializeLiteral(T value, std::vector<uint8_t>& data) {
  SerializeData(reinterpret_cast<const uint8_t*>(&value), sizeof(value), data);
}

void SerializeString(std::string_view s, std::vector<uint8_t>& data);

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
  explicit Socket(int fd);
  virtual ~Socket();

  bool Send(uint32_t kind, base::span<uint8_t> payload);
  std::tuple<std::vector<uint8_t>, uint32_t, bool> Receive();

  void ShutdownWrite();

  int fd_;

 private:
  Socket(const Socket&) = delete;
  Socket& operator=(const Socket&) = delete;
};

class ClientSocket : public Socket {
 public:
  static std::unique_ptr<ClientSocket> Connect(int port);
 private:
  explicit ClientSocket(int fd);
};

class ServerSocket {
 public:
  virtual ~ServerSocket();
  static std::unique_ptr<ServerSocket> Listen(int port);
  std::unique_ptr<Socket> Accept();
 private:
  explicit ServerSocket(int fd);
  int fd_;
  ServerSocket(const ServerSocket&) = delete;
  ServerSocket& operator=(const ServerSocket&) = delete;
};

}  // namespace util

#endif  // UTIL_SOCKET_H_
