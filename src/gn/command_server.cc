// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <chrono>
#include <cstdlib>
#include <memory>
#include <string>
#include <vector>

#include "base/command_line.h"
#include "base/containers/span.h"
#include "base/files/file_util.h"
#include "base/strings/string_number_conversions.h"
#include "base/strings/string_util.h"
#include "gn/commands.h"
#include "gn/filesystem_utils.h"
#include "gn/setup.h"
#include "gn/standard_out.h"
#include "gn/vector_utils.h"
#include "util/socket.h"
#include "util/ticks.h"

namespace commands {

namespace {
// Same as Scheduler::Log but doesn't get forwarded to the client.
void ServerLog(std::string_view verb, std::string_view msg = "") {
  OutputStringLocal(verb, TextDecoration::DECORATION_YELLOW);
  OutputStringLocal(" ");
  OutputStringLocal(msg);
  OutputStringLocal("\n");
}

class CachedSetup : public Setup {
 public:
  CachedSetup(bool verbose) : verbose_(verbose) {}

  void VerboseLog(std::string_view verb, std::string_view msg = "") {
    if (verbose()) {
      ServerLog(verb, msg);
    }
  }

  bool DoSetup(const std::string& build_dir, bool force_create) override {
    return DoSetup(build_dir, force_create,
                   *base::CommandLine::ForCurrentProcess());
  }

  // Reconstructs the setup object.
  void Reset(const char* reason) {
    // Even though it *should* be fine, it is UB to access a field once it's
    // destructed.
    bool verbose = verbose_;
    // In-place destruct and construct preserves pointers to the Setup object.
    this->~CachedSetup();
    new (this) CachedSetup(verbose);
    VerboseLog("Build graph invalidated:", reason);
  }

  bool DoSetup(const std::string& build_dir,
               bool force_create,
               const base::CommandLine& cmdline) override {
    // Explicitly only cache successes, since failures will likely print error
    // messages to the client.
    if (!last_build_dir_.empty() && last_build_dir_ == build_dir &&
        setup_result_) {
      if (IsGraphStale()) {
        Reset("Build graph inputs have changed");
      } else {
        VerboseLog("setup->DoSetup(...)", "cached");
        return true;
      }
    } else if (!last_build_dir_.empty()) {
      Reset("Output directory changed");
    }
    last_build_dir_ = build_dir;
    // Use wall time in nanoseconds to match file modification times from stat.
    // TicksNow() is not the same as the Ticks object returned by mtime.
    last_seen_ = std::chrono::duration_cast<std::chrono::nanoseconds>(
                     std::chrono::system_clock::now().time_since_epoch())
                     .count();
    setup_result_ = Setup::DoSetup(build_dir, force_create, cmdline);
    return setup_result_;
  }

  bool Run() override { return Run(*base::CommandLine::ForCurrentProcess()); }

  bool Run(const base::CommandLine& cmdline) override {
    // Explicitly only cache successes, since failures will likely print error
    // messages to the client.
    VerboseLog("setup->Run(...)", run_result_ ? "cached" : "uncached");
    if (!run_result_) {
      run_result_ = Setup::Run(cmdline);
    }
    return run_result_;
  }

  bool verbose() { return verbose_; }

  void SetWorkingDirectory(const base::FilePath& dir) {
    base::FilePath current;
    base::GetCurrentDirectory(&current);
    if (current != dir) {
      base::SetCurrentDirectory(dir);
      Reset("Working directory changed");
    }
  }

  // Checks if any input to the build graph has been modified since the given
  // time.
  bool IsGraphStale() {
    if (!last_seen_) {
      return false;
    }
    const InputFileManager* input_file_manager =
        scheduler().input_file_manager();
    std::vector<base::FilePath> other_files = scheduler().GetGenDependencies();

    VectorSetSorter<base::FilePath> sorter(
        input_file_manager->GetInputFileCount() + other_files.size());

    input_file_manager->AddAllPhysicalInputFileNamesToVectorSetSorter(&sorter);
    sorter.Add(other_files.begin(), other_files.end());

    for (const auto& file : sorter.AsVector()) {
      base::File::Info file_info;
      if (!base::GetFileInfo(file, &file_info) ||
          file_info.last_modified > *last_seen_) {
        return true;
      }
    }
    return false;
  }

 private:
  std::string last_build_dir_;
  std::optional<Ticks> last_seen_;
  bool setup_result_ = false;
  bool run_result_ = false;
  bool verbose_;
};

Err HandleClientConnection(util::Socket* socket, CachedSetup* setup) {
  setup->VerboseLog("Accepted connection from client");
  auto [payload, kind, success] = socket->Receive();
  if (!success || kind != ServerProtocol::kRunCommand) {
    return Err(Location(), "First command should have been kRunCommand.");
  }

  base::span<uint8_t> span(payload.data(), payload.size());

  auto cwd = util::DeserializeString(span);
  if (!cwd) {
    return Err(Location(), "Failed to deserialize working directory");
  }
  setup->SetWorkingDirectory(UTF8ToFilePath(*cwd));

  int* argc = util::DeserializeLiteral<int>(span);
  if (!argc) {
    return Err(Location(), "Failed to deserialize argc");
  }

  std::vector<const char*> argv(*argc);
  std::vector<std::string_view> argv_sv(*argc);
  for (int i = 0; i < *argc; ++i) {
    auto str = util::DeserializeString(span);
    if (!str) {
      return Err(Location(), "Failed to deserialize argv");
    }
    argv[i] = str->data();
    argv_sv[i] = *str;
  }

  // Reset the command-line and switches so that each command doesn't inherit
  // from the server's context.
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
    ServerLog("\nCommand completed successfully");
  } else {
    ServerLog("\nCommand failed with exit code", std::to_string(return_code));
  }

  SetOutputStringOverride(std::nullopt);

  std::vector<uint8_t> return_code_data;
  util::SerializeLiteral(return_code, return_code_data);
  socket->Send(ServerProtocol::kReturnCode, base::make_span(return_code_data));

  return Err();
}

}  // namespace

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
