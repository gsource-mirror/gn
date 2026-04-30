// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "util/socket.h"

#if defined(OS_WIN)
#include <winsock2.h>
#include <ws2tcpip.h>
#else
#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>
#endif

bool EnsureSockInitialized() {
#if defined(OS_WIN)
  static bool initialized = false;
  if (initialized)
    return true;
  WSADATA wsa_data;
  if (WSAStartup(MAKEWORD(2, 2), &wsa_data) != 0) {
    return false;
  }
  initialized = true;
#endif
  return true;
}

namespace util {

// This could feasibly get quite large, as the requests will likely come in
// batches. While we have sufficient QPS to handle these trivially, the first
// one which is uncached may take a few seconds, meaning that we need to have
// enough space in the queue for the whole batch.
constexpr int kListenQueueSize = 500;

void SerializeString(std::string_view s, std::vector<uint8_t>& data) {
  SerializeLiteral(s.size(), data);
  SerializeData(reinterpret_cast<const uint8_t*>(s.data()), s.size(), data);
  // We manually pad the string with a zero. This is not required, but makes
  // them valid c-style strings.
  data.push_back(0);
}

std::optional<std::string_view> DeserializeString(base::span<uint8_t>& span) {
  size_t* size = DeserializeLiteral<size_t>(span);
  if (!size || *size + 1 > span.size()) {
    return std::nullopt;
  }
  std::string_view val(reinterpret_cast<const char*>(span.data()), *size);
  // Skip over the zero byte as well.
  span = span.subspan(*size + 1);
  return val;
}

Socket::Socket(SocketType fd) : fd_(fd) {}

Socket::~Socket() {
  if (auto fd = this->fd(); fd) {
#if defined(OS_WIN)
    closesocket(*fd);
#else
    close(*fd);
#endif
  }
}

bool Socket::Send(uint32_t kind, base::span<uint8_t> payload) const {
  uint32_t len = static_cast<uint32_t>(payload.size());

  auto send_lambda = [this](const uint8_t* data, size_t size) {
    size_t total_sent = 0;
    while (total_sent < size) {
#if defined(OS_WIN)
      int sent =
          send(this->fd_, reinterpret_cast<const char*>(data + total_sent),
               size - total_sent, 0);
#else
      ssize_t sent = send(this->fd_, data + total_sent, size - total_sent, 0);
#endif
      if (sent <= 0)
        return false;
      total_sent += sent;
    }
    return true;
  };

  if (!send_lambda(reinterpret_cast<const uint8_t*>(&len), sizeof(len)))
    return false;
  if (!send_lambda(reinterpret_cast<const uint8_t*>(&kind), sizeof(kind)))
    return false;
  if (len > 0 && !send_lambda(payload.data(), payload.size()))
    return false;
  return true;
}

std::tuple<std::vector<uint8_t>, uint32_t, bool> Socket::Receive() const {
  auto recv_lambda = [this](uint8_t* data, size_t size) {
    size_t total_read = 0;
    while (total_read < size) {
#if defined(OS_WIN)
      int res = recv(this->fd_, reinterpret_cast<char*>(data + total_read),
                     size - total_read, 0);
#else
      ssize_t res = recv(this->fd_, data + total_read, size - total_read, 0);
#endif
      if (res <= 0)
        return false;
      total_read += res;
    }
    return true;
  };

  uint32_t len = 0;
  if (!recv_lambda(reinterpret_cast<uint8_t*>(&len), sizeof(len)))
    return {{}, 0, false};
  uint32_t kind = 0;
  if (!recv_lambda(reinterpret_cast<uint8_t*>(&kind), sizeof(kind)))
    return {{}, 0, false};
  std::vector<uint8_t> payload(len);
  if (len > 0 && !recv_lambda(payload.data(), len))
    return {{}, kind, false};

  return {std::move(payload), kind, true};
}

std::optional<SocketType> Socket::fd() const {
#if defined(OS_WIN)
  if (fd_ == INVALID_SOCKET)
    return std::nullopt;
#else
  if (fd_ < 0)
    return std::nullopt;
#endif
  return fd_;
}

std::unique_ptr<Socket> Socket::Connect(int port) {
  if (!EnsureSockInitialized())
    return nullptr;
  auto sock = std::make_unique<Socket>(socket(AF_INET, SOCK_STREAM, 0));
  auto fd = sock->fd();
  if (!fd) {
    return nullptr;
  }

  struct sockaddr_in serv_addr;
  serv_addr.sin_family = AF_INET;
  serv_addr.sin_port = htons(port);
  serv_addr.sin_addr.s_addr = inet_addr("127.0.0.1");

  int result = connect(*fd, (struct sockaddr*)&serv_addr, sizeof(serv_addr));
  if (result < 0) {
    return nullptr;
  }
  return sock;
}

std::unique_ptr<ServerSocket> ServerSocket::Listen(int port) {
  if (!EnsureSockInitialized())
    return nullptr;
  // Create a TCP ipv4 socket.
  auto sock = std::make_unique<ServerSocket>(socket(AF_INET, SOCK_STREAM, 0));
  auto fd = sock->fd();
  if (!fd) {
    return nullptr;
  }

  // Ensure you can only connect from localhost.
  struct sockaddr_in address = {};
  address.sin_family = AF_INET;
  address.sin_addr.s_addr = inet_addr("127.0.0.1");
  address.sin_port = htons(port);

  // Bind the socket to the address and port we specified.
  if (bind(*fd, (struct sockaddr*)&address, sizeof(address)) < 0) {
    return nullptr;
  }

  if (listen(*fd, kListenQueueSize) < 0) {
    return nullptr;
  }

  return sock;
}

std::unique_ptr<Socket> ServerSocket::Accept() {
  struct sockaddr_in address;
  socklen_t addrlen = sizeof(address);
  auto sock = std::make_unique<Socket>(
      accept(fd_, (struct sockaddr*)&address, &addrlen));
  if (!sock->fd()) {
    return nullptr;
  }
  return sock;
}

}  // namespace util
