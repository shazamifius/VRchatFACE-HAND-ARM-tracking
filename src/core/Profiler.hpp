#pragma once

#include <chrono>
#include <iostream>
#include <mutex>
#include <numeric>
#include <string>
#include <unordered_map>
#include <vector>

namespace Core {

class Profiler {
public:
  static Profiler &Get() {
    static Profiler instance;
    return instance;
  }

  struct ScopeTimer {
    std::string name;
    std::chrono::high_resolution_clock::time_point start;

    ScopeTimer(std::string n)
        : name(std::move(n)), start(std::chrono::high_resolution_clock::now()) {
    }
    ~ScopeTimer() {
      auto end = std::chrono::high_resolution_clock::now();
      long long duration =
          std::chrono::duration_cast<std::chrono::microseconds>(end - start)
              .count();
      Profiler::Get().Record(name, duration);
    }
  };

  void Record(const std::string &name, long long duration_us) {
    std::lock_guard<std::mutex> lock(mutex_);
    stats_[name].push_back(duration_us);
  }

  void PrintStats() {
    std::lock_guard<std::mutex> lock(mutex_);
    std::cout << "--- Profiler Stats (us) ---\n";
    for (const auto &[name, samples] : stats_) {
      if (samples.empty())
        continue;
      long long sum = std::accumulate(samples.begin(), samples.end(), 0LL);
      double avg = static_cast<double>(sum) / samples.size();
      std::cout << name << ": Avg=" << avg << "us | Runs=" << samples.size()
                << "\n";
    }
    std::cout << "---------------------------\n";
  }

private:
  std::unordered_map<std::string, std::vector<long long>> stats_;
  std::mutex mutex_;
};

} // namespace Core

#define PROFILE_SCOPE(name) Core::Profiler::ScopeTimer timer##__LINE__(name)
