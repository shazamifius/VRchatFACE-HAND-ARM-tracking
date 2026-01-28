#pragma once

#include <iostream>
#include <string>
#include <vector>

#include <winsock2.h>
#include <ws2tcpip.h>

#pragma comment(lib, "Ws2_32.lib")

namespace Network {

class OSCClient {
public:
  OSCClient(const std::string &ip, int port) {
    // Initialize Winsock
    WSADATA wsaData;
    if (WSAStartup(MAKEWORD(2, 2), &wsaData) != 0) {
      std::cerr << "[Network] WSAStartup failed.\n";
      return;
    }

    // Create Socket
    sockfd_ = socket(AF_INET, SOCK_DGRAM, 0);
    if (sockfd_ == INVALID_SOCKET) {
      std::cerr << "[Network] Socket creation failed.\n";
      return;
    }

    // Address setup
    server_addr_.sin_family = AF_INET;
    server_addr_.sin_port = htons(port);
    inet_pton(AF_INET, ip.c_str(), &server_addr_.sin_addr);

    std::cout << "[Network] OSC Client initialized to " << ip << ":" << port
              << "\n";
  }

  ~OSCClient() {
    if (sockfd_ != INVALID_SOCKET) {
      closesocket(sockfd_);
    }
    WSACleanup();
  }

  void Send(const std::vector<char> &packet) {
    if (sockfd_ == INVALID_SOCKET)
      return;

    int sent =
        sendto(sockfd_, packet.data(), static_cast<int>(packet.size()), 0,
               (struct sockaddr *)&server_addr_, sizeof(server_addr_));

    if (sent == SOCKET_ERROR) {
      // std::cerr << "[Network] Send failed: " << WSAGetLastError() << "\n";
    }
  }

  // Helper: Send a single float OSC message
  void SendFloat(const std::string &address, float value) {
    std::vector<char> packet = BuildOSCFloatMessage(address, value);
    Send(packet);
  }

private:
  // Build simple OSC message with float argument
  std::vector<char> BuildOSCFloatMessage(const std::string &address,
                                         float value) {
    std::vector<char> packet;

    // OSC Address
    packet.insert(packet.end(), address.begin(), address.end());
    packet.push_back('\0'); // Null terminator

    // Align to 4-byte boundary
    while (packet.size() % 4 != 0) {
      packet.push_back('\0');
    }

    // OSC Type Tag String
    packet.push_back(',');  // Type tag introducer
    packet.push_back('f');  // Float type
    packet.push_back('\0'); // Null terminator
    packet.push_back('\0'); // Padding to 4-byte alignment

    // Float argument (big-endian)
    union {
      float f;
      uint32_t i;
    } u;
    u.f = value;

    // Convert to big-endian
    uint32_t big_endian = htonl(u.i);
    const char *bytes = reinterpret_cast<const char *>(&big_endian);
    packet.insert(packet.end(), bytes, bytes + 4);

    return packet;
  }
  SOCKET sockfd_ = INVALID_SOCKET;
  struct sockaddr_in server_addr_;
};

} // namespace Network
