#pragma once

#include <atomic>
#include <chrono>
#include <mutex>

#include "biomech/BlendshapeTypes.hpp"

namespace Network {

/**
 * TrackingReceiver - Receives ARKit-compatible blendshapes from phone
 * Similar to VideoReceiver but for JSON tracking data instead of video frames
 */
class TrackingReceiver {
public:
  TrackingReceiver() : has_data_(false), last_update_time_(0) {}

  // Called by WebServer when tracking data arrives
  void OnTrackingDataReceived(const Biomech::ARKitBlendshapes &blendshapes) {
    std::lock_guard<std::mutex> lock(mutex_);
    latest_blendshapes_ = blendshapes;
    has_data_ = true;
    last_update_time_ = std::chrono::duration_cast<std::chrono::milliseconds>(
                            std::chrono::system_clock::now().time_since_epoch())
                            .count();
  }

  // Get latest blendshapes (called by main tracking loop)
  bool GetLatestBlendshapes(Biomech::ARKitBlendshapes &out_blendshapes) {
    std::lock_guard<std::mutex> lock(mutex_);

    // Check if data is fresh (less than 500ms old)
    auto now = std::chrono::duration_cast<std::chrono::milliseconds>(
                   std::chrono::system_clock::now().time_since_epoch())
                   .count();

    if (has_data_ && (now - last_update_time_) < 500) {
      out_blendshapes = latest_blendshapes_;
      return true;
    }

    return false;
  }

  // Check if phone is currently connected (data received in last 3 seconds)
  bool IsConnected() const {
    std::lock_guard<std::mutex> lock(mutex_);

    auto now = std::chrono::duration_cast<std::chrono::milliseconds>(
                   std::chrono::system_clock::now().time_since_epoch())
                   .count();

    return has_data_ && (now - last_update_time_) < 3000;
  }

  // Get time since last update in milliseconds
  long long GetTimeSinceLastUpdate() const {
    std::lock_guard<std::mutex> lock(mutex_);

    auto now = std::chrono::duration_cast<std::chrono::milliseconds>(
                   std::chrono::system_clock::now().time_since_epoch())
                   .count();

    return now - last_update_time_;
  }

private:
  mutable std::mutex mutex_;
  Biomech::ARKitBlendshapes latest_blendshapes_;
  std::atomic<bool> has_data_;
  long long last_update_time_;
};

} // namespace Network
