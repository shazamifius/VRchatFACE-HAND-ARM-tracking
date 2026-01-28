#pragma once

#include <atomic>
#include <iostream>
#include <mutex>
#include <opencv2/opencv.hpp>
#include <vector>


namespace Network {

class VideoReceiver {
public:
  VideoReceiver() = default;

  // Called by WebSocket handler when binary data arrives
  void OnDataReceived(const char *data, size_t length) {
    // We assume each message is a full JPEG frame (from canvas.toBlob)
    // Copy to local buffer to minimize locking time?
    // Or decode immediately? Decoding is heavy, maybe push raw bytes to a
    // queue? For simplicity: Decode immediately (might block receiving thread,
    // but flow control is good)

    try {
      // Need a vector for imdecode
      // Only convert if we can
      if (length > 0) {
        std::vector<uchar> raw(data, data + length);
        cv::Mat decoded = cv::imdecode(raw, cv::IMREAD_COLOR);

        if (!decoded.empty()) {
          std::lock_guard<std::mutex> lock(frames_mutex_);
          latest_frame_ = decoded;
          new_frame_ready_ = true;
          last_received_ts_ = std::chrono::steady_clock::now();
        }
      }
    } catch (...) {
      std::cerr << "[VideoReceiver] Error decoding frame" << std::endl;
    }
  }

  // Called by Vision Thread to get the latest image
  bool GetLatestFrame(cv::Mat &out_frame) {
    std::lock_guard<std::mutex> lock(frames_mutex_);
    if (new_frame_ready_ && !latest_frame_.empty()) {
      latest_frame_.copyTo(out_frame);
      new_frame_ready_ = false; // Consumer consumes it
      return true;
    }
    return false;
  }

  bool IsConnected() const {
    // Simple timeout check: if no frame for 2 seconds, disconnected
    auto now = std::chrono::steady_clock::now();
    auto diff = std::chrono::duration_cast<std::chrono::milliseconds>(
                    now - last_received_ts_)
                    .count();
    return diff < 2000;
  }

private:
  std::mutex frames_mutex_;
  cv::Mat latest_frame_;
  bool new_frame_ready_ = false;
  std::chrono::steady_clock::time_point last_received_ts_;
};

} // namespace Network
