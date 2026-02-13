#pragma once

#include <chrono>
#include <iostream>
#include <numeric>
#include <string>
#include <vector>

namespace Vision {

enum class QualityMode { Ultra, Balanced, Eco, Unknown };

class ModelSelector {
public:
  ModelSelector() = default;

  QualityMode GetCurrentMode() const { return current_mode_; }

  std::string GetModeString() const {
    switch (current_mode_) {
    case QualityMode::Ultra:
      return "Ultra";
    case QualityMode::Balanced:
      return "Balanced";
    case QualityMode::Eco:
      return "Eco";
    default:
      return "Unknown";
    }
  }

private:
  enum class State { Warmup, Benchmark, Stable, Adaptation };

  QualityMode current_mode_ = QualityMode::Balanced; // Default
  State state_ = State::Warmup;
  std::vector<long long> history_;
  int warmup_frames_ = 0;

public:
  void UpdateLatency(long long latency_us) {
    if (state_ == State::Warmup) {
      warmup_frames_++;
      if (warmup_frames_ > 3) {
        state_ = State::Benchmark;
        history_.clear();
      }
      return;
    }

    history_.push_back(latency_us);

    // Limit history size
    if (history_.size() > 60) {
      history_.erase(history_.begin());
    }

    if (state_ == State::Benchmark) {
      if (history_.size() >= 10) {
        DecideMode();
      }
    } else if (state_ == State::Stable || state_ == State::Adaptation) {
      // Continuous monitoring
      // Calculate moving average of last 5 frames
      if (history_.size() >= 5) {
        long long sum = 0;
        for (size_t i = history_.size() - 5; i < history_.size(); ++i)
          sum += history_[i];
        double avg_ms = (sum / 5.0) / 1000.0;

        if (avg_ms > 20.0 && current_mode_ != QualityMode::Eco) {
          current_mode_ = QualityMode::Eco;
          std::cout << "[Adaptive] Downgrading to Eco (Avg: " << avg_ms
                    << "ms)\n";
        } else if (avg_ms < 10.0 && current_mode_ == QualityMode::Eco) {
          current_mode_ = QualityMode::Balanced; // Upgrade conservatively
          std::cout << "[Adaptive] Upgrading to Balanced (Avg: " << avg_ms
                    << "ms)\n";
        }
      }
    }
  }

private:
  void DecideMode() {
    long long sum = std::accumulate(history_.begin(), history_.end(), 0LL);
    double avg_ms = (static_cast<double>(sum) / history_.size()) / 1000.0;

    if (avg_ms < 12.0) {
      current_mode_ = QualityMode::Ultra;
    } else if (avg_ms < 18.0) {
      current_mode_ = QualityMode::Balanced;
    } else {
      current_mode_ = QualityMode::Eco;
    }

    state_ = State::Stable;
    std::cout << "ModelSelector Decision: " << GetModeString()
              << " (Avg Latency: " << avg_ms << "ms)\n";
  }
};

} // namespace Vision
