#pragma once

#include <atomic>
#include <filesystem>
#include <fstream>
#include <httplib.h>
#include <iostream>
#include <mutex>
#include <thread>
#include <vector>

// JSON parsing
#include <nlohmann/json.hpp>

// Only if not defined elsewhere
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <Windows.h>
#include <iphlpapi.h>
#pragma comment(lib, "iphlpapi.lib")

namespace Network {

class VideoReceiver;
class TrackingReceiver;

class WebServer {
public:
  WebServer(int port = 8080) : port_(port), running_(false) {}

  ~WebServer() { Stop(); }

  // Start the server in a separate thread
  bool Start(const std::string &assets_path) {
    if (running_)
      return true;

    svr_.Get("/", [&](const httplib::Request &, httplib::Response &res) {
      std::string file_path = assets_path + "/index.html";
      std::ifstream file(file_path, std::ios::binary);
      if (file) {
        std::string content((std::istreambuf_iterator<char>(file)),
                            std::istreambuf_iterator<char>());
        res.set_content(content, "text/html");
      } else {
        res.set_content(
            "<h1>404 Not Found</h1><p>Could not load index.html</p>",
            "text/html");
      }
    });

    // POST endpoint pour recevoir les frames vidéo du téléphone
    svr_.Post("/video", [&](const httplib::Request &req,
                            httplib::Response &res) {
      if (!req.body.empty() && req.has_header("Content-Type")) {
        std::string ct = req.get_header_value("Content-Type");
        if (ct.find("image/jpeg") != std::string::npos ||
            ct.find("application/octet-stream") != std::string::npos) {
          // Envoyer au VideoReceiver
          if (video_receiver_) {
            video_receiver_->OnDataReceived(req.body.data(), req.body.size());
          }
          res.set_content("OK", "text/plain");
          return;
        }
      }
      res.status = 400;
      res.set_content("Bad Request", "text/plain");
    });

    // OPTIONS endpoint to handle CORS preflight for any route
    svr_.Options(
        "/(.*)", [&](const httplib::Request &, httplib::Response &res) {
          res.set_header("Access-Control-Allow-Origin", "*");
          res.set_header("Access-Control-Allow-Methods", "POST, GET, OPTIONS");
          res.set_header("Access-Control-Allow-Headers", "Content-Type");
          res.status = 204; // No Content
        });

    // POST endpoint pour recevoir les données de tracking MediaPipe du
    // téléphone
    svr_.Post("/tracking",
              [&](const httplib::Request &req, httplib::Response &res) {
                // Always set CORS headers for the actual response too
                res.set_header("Access-Control-Allow-Origin", "*");
                res.set_header("Access-Control-Allow-Methods", "POST, OPTIONS");
                res.set_header("Access-Control-Allow-Headers", "Content-Type");

                if (!req.body.empty() && req.has_header("Content-Type")) {
                  std::string ct = req.get_header_value("Content-Type");
                  if (ct.find("application/json") != std::string::npos) {
                    try {
                      // Parse JSON blendshapes
                      auto json = nlohmann::json::parse(req.body);

                      // Convert JSON to ARKitBlendshapes
                      Biomech::ARKitBlendshapes blendshapes;

                      // Eyes
                      if (json.contains("eyeBlinkLeft"))
                        blendshapes.eyeBlinkLeft = json["eyeBlinkLeft"];
                      if (json.contains("eyeBlinkRight"))
                        blendshapes.eyeBlinkRight = json["eyeBlinkRight"];
                      if (json.contains("eyeSquintLeft"))
                        blendshapes.eyeSquintLeft = json["eyeSquintLeft"];
                      if (json.contains("eyeSquintRight"))
                        blendshapes.eyeSquintRight = json["eyeSquintRight"];
                      if (json.contains("eyeWideLeft"))
                        blendshapes.eyeWideLeft = json["eyeWideLeft"];
                      if (json.contains("eyeWideRight"))
                        blendshapes.eyeWideRight = json["eyeWideRight"];

                      // Jaw
                      if (json.contains("jawOpen"))
                        blendshapes.jawOpen = json["jawOpen"];
                      if (json.contains("jawForward"))
                        blendshapes.jawForward = json["jawForward"];
                      if (json.contains("jawLeft"))
                        blendshapes.jawLeft = json["jawLeft"];
                      if (json.contains("jawRight"))
                        blendshapes.jawRight = json["jawRight"];

                      // Mouth
                      if (json.contains("mouthSmileLeft"))
                        blendshapes.mouthSmileLeft = json["mouthSmileLeft"];
                      if (json.contains("mouthSmileRight"))
                        blendshapes.mouthSmileRight = json["mouthSmileRight"];
                      if (json.contains("mouthFrownLeft"))
                        blendshapes.mouthFrownLeft = json["mouthFrownLeft"];
                      if (json.contains("mouthFrownRight"))
                        blendshapes.mouthFrownRight = json["mouthFrownRight"];
                      if (json.contains("mouthPucker"))
                        blendshapes.mouthPucker = json["mouthPucker"];
                      if (json.contains("mouthFunnel"))
                        blendshapes.mouthFunnel = json["mouthFunnel"];

                      // Brows
                      if (json.contains("browInnerUp"))
                        blendshapes.browInnerUp = json["browInnerUp"];
                      if (json.contains("browOuterUpLeft"))
                        blendshapes.browOuterUpLeft = json["browOuterUpLeft"];
                      if (json.contains("browOuterUpRight"))
                        blendshapes.browOuterUpRight = json["browOuterUpRight"];
                      if (json.contains("browDownLeft"))
                        blendshapes.browDownLeft = json["browDownLeft"];
                      if (json.contains("browDownRight"))
                        blendshapes.browDownRight = json["browDownRight"];

                      // Send to TrackingReceiver
                      if (tracking_receiver_) {
                        tracking_receiver_->OnTrackingDataReceived(blendshapes);
                      }

                      res.status = 200;
                      res.set_content("OK", "text/plain");
                      return;
                    } catch (const std::exception &e) {
                      std::cerr << "[WebServer] JSON parse error: " << e.what()
                                << std::endl;
                      res.status = 400;
                      res.set_content("Invalid JSON", "text/plain");
                      return;
                    }
                  }
                }
                res.status = 400;
                res.set_content("Bad Request", "text/plain");
              });

    // WebSocket endpoint for video data
    svr_.set_mount_point(
        "/video",
        "./"); // Dummy mount for WS? libhttplib handles WS separate?
    // Actually httplib handles WS via callback mechanism usually separate
    // from Get? Let's check typical usage. With current cpp-httplib, we bind
    // listeners.

    // Note: httplib needs a thread to run listen.

    running_ = true;
    server_thread_ = std::thread([this]() {
      std::cout << "[WebServer] Starting on port " << port_ << "..."
                << std::endl;

      // On connect/message placeholders
      // To properly handle MJPEG stream, we need to collect chunks.
      // Client sends full blobs (JPEGs).

      // NOTE: Implementing WebSocket with raw httplib depends on the version.
      // Older versions didn't have full WS support or required specific
      // flags. Assuming v0.14+ (vcpkg usually has recent).

      // Since we need to push data to the main thread, we'll store it in a
      // shared buffer But how do we define the WS handler? "svr_.Get" is
      // HTTP. "svr_.set_websocket_callback" is usually the way? Let's assume
      // standard modern httplib usage.

      // For now, let's just make sure it compiles and serves HTML.
      // We will implement VideoReceiver separately or integrate here.

      if (!svr_.listen("0.0.0.0", port_)) {
        std::cerr << "[WebServer] Failed to bind to port " << port_
                  << std::endl;
        running_ = false;
      }
    });

    return true;
  }

  void Stop() {
    if (running_) {
      svr_.stop();
      if (server_thread_.joinable())
        server_thread_.join();
      running_ = false;
      std::cout << "[WebServer] Stopped." << std::endl;
    }
  }

  // Helper to get local IP for QR Code
  std::string GetLocalIP() {
    // Simple Windows implementation
    ULONG outBufLen = 15000;
    PIP_ADAPTER_ADDRESSES pAddresses = (PIP_ADAPTER_ADDRESSES)malloc(outBufLen);
    std::string ip = "127.0.0.1";

    if (GetAdaptersAddresses(AF_INET, GAA_FLAG_INCLUDE_PREFIX, NULL, pAddresses,
                             &outBufLen) == NO_ERROR) {
      PIP_ADAPTER_ADDRESSES pCurrAddresses = pAddresses;
      while (pCurrAddresses) {
        if (pCurrAddresses->OperStatus == IfOperStatusUp &&
            pCurrAddresses->IfType != IF_TYPE_SOFTWARE_LOOPBACK) {
          PIP_ADAPTER_UNICAST_ADDRESS pUnicast =
              pCurrAddresses->FirstUnicastAddress;
          if (pUnicast) {
            char buffer[100];
            DWORD size = sizeof(buffer);
            if (WSAAddressToStringA(pUnicast->Address.lpSockaddr,
                                    pUnicast->Address.iSockaddrLength, NULL,
                                    buffer, &size) == 0) {
              ip = buffer;
              break; // Take first valid
            }
          }
        }
        pCurrAddresses = pCurrAddresses->Next;
      }
    }
    free(pAddresses);
    return ip;
  }

  int GetPort() const { return port_; }

  // Injecter la référence au VideoReceiver
  void SetVideoReceiver(Network::VideoReceiver *receiver) {
    video_receiver_ = receiver;
  }

  // Injecter la référence au TrackingReceiver
  void SetTrackingReceiver(Network::TrackingReceiver *receiver) {
    tracking_receiver_ = receiver;
  }

private:
  httplib::Server svr_;
  int port_;
  std::atomic<bool> running_;
  std::thread server_thread_;
  Network::VideoReceiver *video_receiver_ = nullptr;
  Network::TrackingReceiver *tracking_receiver_ = nullptr;
};

} // namespace Network
