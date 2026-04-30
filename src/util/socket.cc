// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "util/socket.h"

#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>

namespace util {

void SerializeString(std::string_view s, std::vector<uint8_t>& data) {
  SerializeLiteral(s.size(), data);
  SerializeData(reinterpret_cast<const uint8_t*>(s.data()), s.size(), data);
  data.push_back(0);
}

std::optional<std::string_view> DeserializeString(
    base::span<uint8_t>& span) {
  auto size = DeserializeLiteral<size_t>(span);
  if (!size || *size > span.size()) {
    return std::nullopt;
  }
  std::string_view val(reinterpret_cast<const char*>(span.data()), *size);
  span = span.subspan(*size + 1);
  return val;
}

Socket::Socket(int fd) : fd_(fd) {}

Socket::~Socket() {
  if (fd_ != -1) {
    close(fd_);
  }
}

bool Socket::Send(uint32_t kind, base::span<uint8_t> payload) {
  uint32_t len = static_cast<uint32_t>(payload.size());

  auto send = [this](const uint8_t* data, size_t size) {
    size_t total_sent = 0;
    while (total_sent < size) {
      ssize_t sent = write(this->fd_, data + total_sent, size - total_sent);
      if (sent <= 0)
        return false;
      total_sent += sent;
    }
    return true;
  };

  if (!send(reinterpret_cast<const uint8_t*>(&len), sizeof(len)))
    return false;
  if (!send(reinterpret_cast<const uint8_t*>(&kind), sizeof(kind)))
    return false;
  if (len > 0 && !send(payload.data(), payload.size()))
    return false;
  return true;
}

std::tuple<std::vector<uint8_t>, uint32_t, bool> Socket::Receive() {
  auto recv = [this](uint8_t* data, size_t size) {
    size_t total_read = 0;
    while (total_read < size) {
      ssize_t res = read(this->fd_, data + total_read, size - total_read);
      if (res <= 0)
        return false;
      total_read += res;
    }
    return true;
  };

  uint32_t len = 0;
  if (!recv(reinterpret_cast<uint8_t*>(&len), sizeof(len)))
    return {{}, 0, false};
  uint32_t kind = 0;
  if (!recv(reinterpret_cast<uint8_t*>(&kind), sizeof(kind)))
    return {{}, 0, false};
  std::vector<uint8_t> payload(len);
  if (len > 0 && !recv(payload.data(), len))
    return {{}, kind, false};

  return {std::move(payload), kind, true};
}

void Socket::ShutdownWrite() {
  if (fd_ != -1) {
    shutdown(fd_, SHUT_WR);
  }
}

ClientSocket::ClientSocket(int fd) : Socket(fd) {}

std::unique_ptr<ClientSocket> ClientSocket::Connect(int port) {
  int fd = socket(AF_INET, SOCK_STREAM, 0);
  if (fd < 0) {
    return nullptr;
  }

  struct sockaddr_in serv_addr;
  serv_addr.sin_family = AF_INET;
  serv_addr.sin_port = htons(port);
  serv_addr.sin_addr.s_addr = inet_addr("127.0.0.1");

  if (connect(fd, (struct sockaddr*)&serv_addr, sizeof(serv_addr)) < 0) {
    close(fd);
    return nullptr;
  }

  return std::unique_ptr<ClientSocket>(new ClientSocket(fd));
}

ServerSocket::ServerSocket(int fd) : fd_(fd) {}

ServerSocket::~ServerSocket() {
  if (fd_ != -1) {
    close(fd_);
  }
}

std::unique_ptr<ServerSocket> ServerSocket::Listen(int port) {
  int fd = socket(AF_INET, SOCK_STREAM, 0);
  if (fd < 0) {
    return nullptr;
  }

  int opt = 1;
  if (setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) < 0) {
    close(fd);
    return nullptr;
  }

  struct sockaddr_in address;
  address.sin_family = AF_INET;
  address.sin_addr.s_addr = INADDR_ANY;
  address.sin_port = htons(port);

  if (bind(fd, (struct sockaddr*)&address, sizeof(address)) < 0) {
    close(fd);
    return nullptr;
  }

  if (listen(fd, 3) < 0) {
    close(fd);
    return nullptr;
  }

  return std::unique_ptr<ServerSocket>(new ServerSocket(fd));
}

std::unique_ptr<Socket> ServerSocket::Accept() {
  struct sockaddr_in address;
  socklen_t addrlen = sizeof(address);
  int new_socket = accept(fd_, (struct sockaddr*)&address, &addrlen);
  if (new_socket < 0) {
    return nullptr;
  }
  return std::unique_ptr<Socket>(new Socket(new_socket));
}

}  // namespace util
