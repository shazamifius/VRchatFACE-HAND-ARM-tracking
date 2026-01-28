#pragma once

#include <Windows.h>
#include <atomic>
#include <iostream>
#include <regex>
#include <string>
#include <thread>

namespace Network {

class CloudflareTunnel {
public:
  CloudflareTunnel() : running_(false), process_handle_(NULL) {}

  ~CloudflareTunnel() { Stop(); }

  // Start tunnel and return public URL
  bool Start(int local_port) {
    if (running_)
      return true;

    local_port_ = local_port;

    // Check if cloudflared.exe exists
    std::string cloudflared_path = "scripts/cloudflared/cloudflared.exe";
    if (!FileExists(cloudflared_path)) {
      std::cout << "[Cloudflare] cloudflared.exe not found, downloading..."
                << std::endl;
      if (!DownloadCloudflared(cloudflared_path)) {
        std::cerr << "[Cloudflare] Failed to download cloudflared.exe"
                  << std::endl;
        return false;
      }
    }

    std::cout << "[Cloudflare] Starting tunnel on port " << local_port << "..."
              << std::endl;

    // Launch cloudflared in separate thread
    tunnel_thread_ = std::thread(
        [this, cloudflared_path]() { LaunchTunnel(cloudflared_path); });

    // Wait for URL to be parsed (max 10 seconds)
    for (int i = 0; i < 100; i++) {
      if (!public_url_.empty()) {
        std::cout << "[Cloudflare] Tunnel URL: " << public_url_ << std::endl;
        running_ = true;
        return true;
      }
      std::this_thread::sleep_for(std::chrono::milliseconds(100));
    }

    std::cerr << "[Cloudflare] Timeout waiting for tunnel URL" << std::endl;
    Stop();
    return false;
  }

  std::string GetPublicURL() const { return public_url_; }

  void Stop() {
    if (!running_)
      return;

    running_ = false;

    if (process_handle_ != NULL) {
      TerminateProcess(process_handle_, 0);
      CloseHandle(process_handle_);
      process_handle_ = NULL;
    }

    if (tunnel_thread_.joinable()) {
      tunnel_thread_.join();
    }

    std::cout << "[Cloudflare] Tunnel stopped." << std::endl;
  }

private:
  bool FileExists(const std::string &path) {
    DWORD attribs = GetFileAttributesA(path.c_str());
    return (attribs != INVALID_FILE_ATTRIBUTES &&
            !(attribs & FILE_ATTRIBUTE_DIRECTORY));
  }

  bool DownloadCloudflared(const std::string &dest_path) {
    // URL for latest cloudflared Windows binary
    std::string url = "https://github.com/cloudflare/cloudflared/releases/"
                      "latest/download/cloudflared-windows-amd64.exe";

    // Use PowerShell to download
    std::string cmd =
        "powershell -Command \"Invoke-WebRequest -Uri '" + url +
        "' -OutFile '" + dest_path +
        "' -UseBasicParsing; if ($?) { exit 0 } else { exit 1 }\"";

    std::cout << "[Cloudflare] Downloading from GitHub..." << std::endl;

    int result = system(cmd.c_str());
    if (result == 0 && FileExists(dest_path)) {
      std::cout << "[Cloudflare] Download complete: " << dest_path << std::endl;
      return true;
    }

    return false;
  }

  void LaunchTunnel(const std::string &cloudflared_path) {
    // Build command
    std::string cmd = cloudflared_path + " tunnel --url http://localhost:" +
                      std::to_string(local_port_);

    // Setup process pipes
    SECURITY_ATTRIBUTES sa;
    sa.nLength = sizeof(SECURITY_ATTRIBUTES);
    sa.bInheritHandle = TRUE;
    sa.lpSecurityDescriptor = NULL;

    HANDLE stdout_read = NULL;
    HANDLE stdout_write = NULL;

    if (!CreatePipe(&stdout_read, &stdout_write, &sa, 0)) {
      std::cerr << "[Cloudflare] Failed to create pipe" << std::endl;
      return;
    }

    SetHandleInformation(stdout_read, HANDLE_FLAG_INHERIT, 0);

    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    ZeroMemory(&si, sizeof(si));
    si.cb = sizeof(si);
    si.hStdOutput = stdout_write;
    si.hStdError = stdout_write;
    si.dwFlags |= STARTF_USESTDHANDLES | STARTF_USESHOWWINDOW;
    si.wShowWindow = SW_HIDE;
    ZeroMemory(&pi, sizeof(pi));

    // Start process
    if (!CreateProcessA(NULL, const_cast<char *>(cmd.c_str()), NULL, NULL, TRUE,
                        0, NULL, NULL, &si, &pi)) {
      std::cerr << "[Cloudflare] Failed to start process" << std::endl;
      CloseHandle(stdout_read);
      CloseHandle(stdout_write);
      return;
    }

    process_handle_ = pi.hProcess;
    CloseHandle(pi.hThread);
    CloseHandle(stdout_write);

    // Read output to parse URL
    char buffer[4096];
    DWORD bytes_read;
    std::string output;

    while (running_ && public_url_.empty()) {
      if (ReadFile(stdout_read, buffer, sizeof(buffer) - 1, &bytes_read,
                   NULL) &&
          bytes_read > 0) {
        buffer[bytes_read] = '\0';
        output += buffer;

        // Try to parse URL from output
        std::string url = ParseURLFromOutput(output);
        if (!url.empty()) {
          public_url_ = url;
          break;
        }
      } else {
        break;
      }
    }

    CloseHandle(stdout_read);

    // Keep process running
    if (running_) {
      WaitForSingleObject(process_handle_, INFINITE);
    }
  }

  std::string ParseURLFromOutput(const std::string &output) {
    // Cloudflare outputs: "https://random-name.trycloudflare.com"
    // Regex to match this pattern
    std::regex url_regex(R"(https://[a-z0-9-]+\.trycloudflare\.com)",
                         std::regex_constants::icase);
    std::smatch match;

    if (std::regex_search(output, match, url_regex)) {
      std::string full_url = match.str();
      // Remove "https://" to return just the hostname
      if (full_url.find("https://") == 0) {
        return full_url.substr(8);
      }
      return full_url;
    }

    return "";
  }

  std::atomic<bool> running_;
  HANDLE process_handle_;
  std::thread tunnel_thread_;
  std::string public_url_;
  int local_port_;
};

} // namespace Network
