#pragma once

#include "../core/MathUtils.hpp"
#include <chrono>

namespace Biomech {

class OneEuroFilter {
public:
  // Configuration adapted for VR tracking
  struct Params {
    float min_cutoff = 1.0f; // Hz
    float beta = 0.5f;       // Speed coefficient
    float d_cutoff = 1.0f;   // Derivative cutoff
  };

  explicit OneEuroFilter(Params params = Params())
      : params_(params), first_update_(true) {}

  Core::Vector3 Filter(const Core::Vector3 &noisy_val, double timestamp_s) {
    if (first_update_) {
      first_update_ = false;
      x_prev_ = noisy_val;
      dx_prev_ = Core::Vector3(0.0f);
      t_prev_ = timestamp_s;
      return noisy_val;
    }

    double dt = timestamp_s - t_prev_;
    if (dt <= 0.0)
      return x_prev_; // Duplicate frame or time glitch

    float alpha_d = Alpha(dt, params_.d_cutoff);
    Core::Vector3 dx = (noisy_val - x_prev_) / static_cast<float>(dt);
    Core::Vector3 dx_hat = Core::MathUtils::Lerp(dx_prev_, dx, alpha_d);

    float cutoff = params_.min_cutoff + params_.beta * glm::length(dx_hat);
    float alpha = Alpha(dt, cutoff);
    Core::Vector3 x_hat = Core::MathUtils::Lerp(x_prev_, noisy_val, alpha);

    x_prev_ = x_hat;
    dx_prev_ = dx_hat;
    t_prev_ = timestamp_s;

    return x_hat;
  }

  float FilterScalar(float noisy_val, double timestamp_s) {
    if (first_update_) {
      // For scalar, we can just use the Vector3 implementation with y=0, z=0
      // But separate state is cleaner if we want to support both.
      // However, usually one filter instance = one signal.
      // So we can reuse the Vector3 state variables (x_prev_.x)

      first_update_ = false;
      x_prev_ = Core::Vector3(noisy_val, 0.0f, 0.0f);
      dx_prev_ = Core::Vector3(0.0f);
      t_prev_ = timestamp_s;
      return noisy_val;
    }

    double dt = timestamp_s - t_prev_;
    if (dt <= 0.0)
      return x_prev_.x;

    float alpha_d = Alpha(dt, params_.d_cutoff);
    float dx = (noisy_val - x_prev_.x) / static_cast<float>(dt);
    // float dx_hat = Core::MathUtils::Lerp(dx_prev_.x, dx, alpha_d);
    float dx_hat = dx_prev_.x + (dx - dx_prev_.x) * alpha_d;

    float cutoff = params_.min_cutoff + params_.beta * std::abs(dx_hat);
    float alpha = Alpha(dt, cutoff);
    // float x_hat = Core::MathUtils::Lerp(x_prev_.x, noisy_val, alpha);
    float x_hat = x_prev_.x + (noisy_val - x_prev_.x) * alpha;

    x_prev_.x = x_hat;
    dx_prev_.x = dx_hat;
    t_prev_ = timestamp_s;

    return x_hat;
  }

private:
  Params params_;
  bool first_update_;
  Core::Vector3 x_prev_;
  Core::Vector3 dx_prev_;
  double t_prev_;

  float Alpha(double dt, float cutoff) {
    float tau = 1.0f / (2.0f * Core::MathUtils::PI * cutoff);
    return static_cast<float>(1.0 / (1.0 + tau / dt));
  }
};

class Extrapolator {
public:
  // Simple linear extrapolation based on velocity
  static Core::Vector3 Predict(const Core::Vector3 &pos,
                               const Core::Vector3 &velocity,
                               float latency_ms) {
    return pos + velocity * (latency_ms / 1000.0f);
  }
};

} // namespace Biomech
