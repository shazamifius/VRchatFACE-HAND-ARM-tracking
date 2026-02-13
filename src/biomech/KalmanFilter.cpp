#include "KalmanFilter.hpp"

namespace Biomech {

HeadPoseKalmanFilter::HeadPoseKalmanFilter() : initialized_(false) {
  // Initialize all three Kalman filters
  kalman_pitch_ = std::make_unique<cv::KalmanFilter>(InitKalmanForAngle());
  kalman_yaw_ = std::make_unique<cv::KalmanFilter>(InitKalmanForAngle());
  kalman_roll_ = std::make_unique<cv::KalmanFilter>(InitKalmanForAngle());
}

cv::KalmanFilter HeadPoseKalmanFilter::InitKalmanForAngle() {
  /**
   * Kalman Filter Configuration for Head Rotation
   *
   * State: [angle, angular_velocity]
   * Measurement: [angle]
   *
   * Process Model:
   *   x_k = F * x_{k-1} + w  (w ~ N(0, Q))
   *   z_k = H * x_k + v      (v ~ N(0, R))
   */

  cv::KalmanFilter kf(2, 1, 0); // 2 state vars, 1 measurement, 0 control

  // State Transition Matrix F
  // [1  dt]
  // [0   1]
  float dt = 1.0f / 60.0f; // Assume 60 FPS
  kf.transitionMatrix = (cv::Mat_<float>(2, 2) << 1, dt, 0, 1);

  // Measurement Matrix H (we only measure angle, not velocity)
  // [1  0]
  kf.measurementMatrix = (cv::Mat_<float>(1, 2) << 1, 0);

  // Process Noise Covariance Q
  // Tuned for head movement (not too fast, not too slow)
  // Lower values = smoother but more lag
  // Higher values = more responsive but more jitter
  float q_angle = 0.01f;   // Process noise for angle
  float q_velocity = 0.1f; // Process noise for velocity
  kf.processNoiseCov = (cv::Mat_<float>(2, 2) << q_angle, 0, 0, q_velocity);

  // Measurement Noise Covariance R
  // Represents trust in measurements (lower = trust more)
  float r_angle = 0.5f; // Moderate trust (camera can be jittery)
  kf.measurementNoiseCov = (cv::Mat_<float>(1, 1) << r_angle);

  // Error Covariance P (initial uncertainty)
  kf.errorCovPost = (cv::Mat_<float>(2, 2) << 1, 0, 0, 1);

  return kf;
}

SimpleVector3 HeadPoseKalmanFilter::Filter(const SimpleVector3 &rotation_deg) {
  if (!initialized_) {
    // First call: Initialize state with current measurement
    kalman_pitch_->statePost = (cv::Mat_<float>(2, 1) << rotation_deg.x, 0);
    kalman_yaw_->statePost = (cv::Mat_<float>(2, 1) << rotation_deg.y, 0);
    kalman_roll_->statePost = (cv::Mat_<float>(2, 1) << rotation_deg.z, 0);
    initialized_ = true;
    return rotation_deg; // Return unfiltered on first frame
  }

  // Predict step
  cv::Mat pitch_pred = kalman_pitch_->predict();
  cv::Mat yaw_pred = kalman_yaw_->predict();
  cv::Mat roll_pred = kalman_roll_->predict();

  // Update step (correct with measurement)
  cv::Mat pitch_meas = (cv::Mat_<float>(1, 1) << rotation_deg.x);
  cv::Mat yaw_meas = (cv::Mat_<float>(1, 1) << rotation_deg.y);
  cv::Mat roll_meas = (cv::Mat_<float>(1, 1) << rotation_deg.z);

  cv::Mat pitch_est = kalman_pitch_->correct(pitch_meas);
  cv::Mat yaw_est = kalman_yaw_->correct(yaw_meas);
  cv::Mat roll_est = kalman_roll_->correct(roll_meas);

  // Extract filtered angles (first element of state vector)
  SimpleVector3 filtered;
  filtered.x = pitch_est.at<float>(0);
  filtered.y = yaw_est.at<float>(0);
  filtered.z = roll_est.at<float>(0);

  return filtered;
}

void HeadPoseKalmanFilter::Reset() {
  initialized_ = false;
  // Reinitialize filters
  kalman_pitch_ = std::make_unique<cv::KalmanFilter>(InitKalmanForAngle());
  kalman_yaw_ = std::make_unique<cv::KalmanFilter>(InitKalmanForAngle());
  kalman_roll_ = std::make_unique<cv::KalmanFilter>(InitKalmanForAngle());
}

} // namespace Biomech
