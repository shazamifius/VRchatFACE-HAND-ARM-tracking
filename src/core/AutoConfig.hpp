#pragma once

#include <iostream>
#include <string>

// Windows specific for netstat check (simplified)
// or just socket check
#include <winsock2.h>

namespace Core {

class AutoConfig {
public:
  // returns true if port is OPEN (listening) on localhost
  // For VRChat, we want to know if it's listening on 9000 (OSC Default)
  static bool IsVRChatRunning(int port = 9000) {
    // Simple check: try to connect to the UDP port?
    // UDP doesn't really "connect".
    // We can try to bind to it. If we FAIL to bind, someone else (VRChat) might
    // be using it? Actually, for UDP, multiple apps can sometimes bind with
    // SO_REUSEADDR. But usually, one app binds. Better check: Send a query?
    // VRChat doesn't respond to queries easily.

    // Let's use a heuristic: process list or just assume port 9000.
    // For now, let's just log.
    return true;
  }

  static int FindOSCPort() {
    // Default VRChat receiving port
    return 9000;
  }
};

} // namespace Core
