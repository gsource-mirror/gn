// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "base/strings/string_util.h"
#include "gn/commands.h"

#include <cstdlib>
#include <memory>
#include <string>
#include <vector>

#include "base/command_line.h"
#include "base/containers/span.h"
#include "base/strings/string_number_conversions.h"
#include "base/strings/string_split.h"
#include "gn/setup.h"
#include "gn/standard_out.h"
#include "util/msg_loop.h"
#include "util/socket.h"

namespace commands {

class CachedSetup : public Setup {
 public:
  CachedSetup(bool verbose) : verbose_(verbose) {}

  bool DoSetup(const std::string& build_dir, bool force_create) override {
    return DoSetup(build_dir, force_create,
                   *base::CommandLine::ForCurrentProcess());
  }

  bool DoSetup(const std::string& build_dir,
               bool force_create,
               const base::CommandLine& cmdline) override {
    if (!last_build_dir_.empty() && last_build_dir_ == build_dir &&
        setup_result_) {
      return true;
    }
    if (!last_build_dir_.empty()) {
      // Re-initialize everything
      this->~CachedSetup();
      new (this) CachedSetup(verbose_);
    }
    last_build_dir_ = build_dir;
    setup_result_ = Setup::DoSetup(build_dir, force_create, cmdline);
    return setup_result_;
  }

  bool Run() override { return Run(*base::CommandLine::ForCurrentProcess()); }

  bool Run(const base::CommandLine& cmdline) override {
    // Explicitly only cache successes.
    if (!run_result_) {
      run_result_ = Setup::Run(cmdline);
    }
    return run_result_;
  }

  bool verbose() { return verbose_; }

 private:
  std::string last_build_dir_;
  bool setup_result_ = false;
  bool run_result_ = false;
  bool verbose_;
};

// Same as Scheduler.log but doesn't get forwarded to the client and uses
// verbosity from server instead of client.
void ServerLog(std::string_view verb, std::string_view msg = "") {
  OutputStringLocal(verb, TextDecoration::DECORATION_YELLOW);
  OutputStringLocal(" ");
  OutputStringLocal(msg);
  OutputStringLocal("\n");
}

Err HandleClientConnection(util::Socket* socket, CachedSetup* setup) {
  if (setup->verbose()) {
    ServerLog("Accepted connection from client");
  }
  auto [payload, kind, success] = socket->Receive();
  if (!success || kind != ServerProtocol::kRunCommand) {
    return Err(Location(), "First command should have been kRunCommand.");
  }

  base::span<uint8_t> span(payload.data(), payload.size());

  int* argc = util::DeserializeLiteral<int>(span);
  if (!argc) {
    return Err(Location(), "Failed to deserialize argc");
  }

  std::vector<char*> argv(*argc);
  std::vector<std::string_view> argv_sv(*argc);
  for (int i = 0; i < *argc; ++i) {
    auto str = util::DeserializeString(span);
    if (!str) {
      return Err(Location(), "Failed to deserialize argv");
    }
    argv[i] = const_cast<char*>(str->data());
    argv_sv[i] = *str;
  }

  base::CommandLine::Reset();
  CommandSwitches::Set(CommandSwitches());

  SetOutputStringOverride([socket, setup](std::string_view output,
                                          TextDecoration dec,
                                          HtmlEscaping escaping) {
    if (setup->verbose()) {
      OutputStringLocal(output, dec, escaping);
    }
    std::vector<uint8_t> data;
    util::SerializeLiteral(dec, data);
    util::SerializeLiteral(escaping, data);
    util::SerializeString(output, data);
    socket->Send(ServerProtocol::kOutputString, base::make_span(data));
  });

  ServerLog("Running command", base::JoinString(argv_sv, " "));
  int return_code = commands::gn_main(*argc, argv.data(), setup);
  if (return_code == 0) {
    ServerLog("Command completed successfully");
  } else {
    ServerLog("Command failed with exit code", std::to_string(return_code));
  }

  SetOutputStringOverride(std::nullopt);

  std::vector<uint8_t> return_code_data;
  util::SerializeLiteral(return_code, return_code_data);
  socket->Send(ServerProtocol::kReturnCode, base::make_span(return_code_data));

  return Err();
}

int RunServer(Setup* _, const std::vector<std::string>& args) {
  bool verbose = base::CommandLine::ForCurrentProcess()->HasSwitch("v") ||
                 base::CommandLine::ForCurrentProcess()->HasSwitch("verbose");

  if (args.size() != 0) {
    Err(Location(), "gn server requires no arguments").PrintToStdout();
    return 1;
  }

  const char* port_str = std::getenv("GN_PORT");
  if (!port_str) {
    Err(Location(), "GN_PORT environment variable not set.").PrintToStdout();
    return 1;
  }

  int port = 0;
  if (!base::StringToInt(port_str, &port) || port <= 0) {
    Err(Location(), "Invalid GN_PORT value: " + std::string(port_str))
        .PrintToStdout();
    return 1;
  }

  CachedSetup* setup = new CachedSetup(verbose);

  auto server = util::ServerSocket::Listen(port);
  if (!server) {
    Err(Location(), "Could not listen on port " + std::to_string(port))
        .PrintToStdout();
    return 1;
  }

  ServerLog("GN server listening on port", std::to_string(port));

  while (true) {
    auto socket = server->Accept();
    if (!socket)
      continue;

    auto err = HandleClientConnection(socket.get(), setup);
    if (err.has_error()) {
      err.PrintToStdout();
    }
  }

  return 0;
}

const char kServer[] = "server";
const char kServer_HelpShort[] = "server: Run a persistent GN server daemon.";
const char kServer_Help[] =
    R"(server: Run a persistent GN server daemon.

  gn server <out_dir>

  Starts a background daemon that caches the parsed build graph in memory, 
  greatly accelerating subsequent queries from the GN thin-client.
)";

}  // namespace commands
