#pragma once

#include <memory>
#include <opencv2/opencv.hpp>
#include <opencv2/video/tracking.hpp>


namespace Biomech {

// Lightweight Vector3 for Kalman filter (avoids glm dependency in header)
struct SimpleVector3 {
  float x, y, z;
  SimpleVector3() : x(0), y(0), z(0) {}
  SimpleVector3(float _x, float _y, float _z) : x(_x), y(_y), z(_z) {}
};

/**
 * @brief Kalman Filter optimized for head pose rotation tracking
 *
 * Uses separate Kalman filters for Pitch, Yaw, and Roll to provide
 * superior stability compared to OneEuroFilter for angular data.
 * Particularly effective at reducing jitter during head tracking.
 */
class HeadPoseKalmanFilter {
public:
  HeadPoseKalmanFilter();

  /**
   * @brief Filter a 3D rotation vector (Pitch, Yaw, Roll in degrees)
   * @param rotation_deg Input rotation in degrees (x=Pitch, y=Yaw, z=Roll)
   * @return Filtered rotation in degrees
   */
  SimpleVector3 Filter(const SimpleVector3 &rotation_deg);

  /**
   * @brief Reset the filter state (useful when tracking is lost)
   */
  void Reset();

  /**
   * @brief Check if the filter has been initialized
   */
  bool IsInitialized() const { return initialized_; }

private:
  /**
   * @brief Initialize a Kalman filter for a single angle
   * @return Configured cv::KalmanFilter instance
   */
  cv::KalmanFilter InitKalmanForAngle();

  std::unique_ptr<cv::KalmanFilter> kalman_pitch_;
  std::unique_ptr<cv::KalmanFilter> kalman_yaw_;
  std::unique_ptr<cv::KalmanFilter> kalman_roll_;

  bool initialized_;
};

} // namespace Biomech
