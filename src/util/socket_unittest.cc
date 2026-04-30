// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "util/socket.h"

#include <thread>
#include <vector>

#include "util/test/test.h"

#if defined(OS_WIN)
#include <winsock2.h>
#include <ws2tcpip.h>
#else
#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>
#endif

TEST(SocketTest, BasicCommunication) {
  // Just pick an arbitrary free port
  auto server = util::ServerSocket::Listen(0);
  ASSERT_TRUE(server);
  auto fd = server->fd();
  ASSERT_TRUE(fd);

  // Find which port was chosen.
  struct sockaddr_in addr;
  socklen_t len;
  ASSERT_EQ(getsockname(*fd, (struct sockaddr*)&addr, &len), 0);
  int port = ntohs(addr.sin_port);

  std::unique_ptr<util::Socket> server_client_socket;
  std::thread accept_thread([&]() { server_client_socket = server->Accept(); });

  auto client = util::Socket::Connect(port);
  ASSERT_TRUE(client);

  accept_thread.join();
  ASSERT_TRUE(server_client_socket);

  std::vector<uint8_t> data;
  util::SerializeLiteral<int>(42, data);
  util::SerializeString("Hello", data);

  EXPECT_TRUE(client->Send(1, data));

  auto [payload, kind, success] = server_client_socket->Receive();
  EXPECT_TRUE(success);
  EXPECT_EQ(kind, 1u);

  base::span<uint8_t> span(payload);
  int* val = util::DeserializeLiteral<int>(span);
  ASSERT_TRUE(val);
  EXPECT_EQ(*val, 42);

  auto str = util::DeserializeString(span);
  ASSERT_TRUE(str);
  EXPECT_EQ(*str, "Hello");

  // The span should now be fully consumed, so attempting to consume more data
  // from it should fail.
  EXPECT_FALSE(util::DeserializeLiteral<int>(span));
  EXPECT_FALSE(util::DeserializeString(span));
}
