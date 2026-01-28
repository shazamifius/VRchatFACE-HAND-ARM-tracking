#pragma once

#include <atomic>
#include <cstddef>
#include <memory_resource>
#include <new>
#include <optional>
#include <vector>


// Optimization for cache line size (usually 64 bytes)
constexpr size_t CACHE_LINE_SIZE = 64;

namespace Core {

template <typename T, typename Allocator = std::pmr::polymorphic_allocator<T>>
class RingBuffer {
public:
  explicit RingBuffer(size_t capacity, Allocator alloc = {})
      : buffer_(alloc), capacity_(capacity) {
    // Capacity must be power of 2 for efficient wrapping?
    // Or just simple modulo. Power of 2 allows bitwise AND.
    // Let's enforce power of 2 or just use valid modulo.
    // For safety, let's keep it simple first, but aligned.
    buffer_.resize(capacity +
                   1); // +1 to distinguish full/empty if using head/tail
  }

  // Disable copy/move
  RingBuffer(const RingBuffer &) = delete;
  RingBuffer &operator=(const RingBuffer &) = delete;

  bool push(const T &item) {
    const auto current_head = head_.load(std::memory_order_relaxed);
    const auto next_head = (current_head + 1) % buffer_.size();

    if (next_head == tail_.load(std::memory_order_acquire)) {
      return false; // Full
    }

    buffer_[current_head] = item;
    head_.store(next_head, std::memory_order_release);
    return true;
  }

  bool pop(T &out_item) {
    const auto current_tail = tail_.load(std::memory_order_relaxed);

    if (current_tail == head_.load(std::memory_order_acquire)) {
      return false; // Empty
    }

    out_item = buffer_[current_tail];
    tail_.store((current_tail + 1) % buffer_.size(), std::memory_order_release);
    return true;
  }

  bool empty() const {
    return head_.load(std::memory_order_acquire) ==
           tail_.load(std::memory_order_acquire);
  }

  // Add move semantics support to push if T is movable
  bool push(T &&item) {
    const auto current_head = head_.load(std::memory_order_relaxed);
    const auto next_head = (current_head + 1) % buffer_.size();

    if (next_head == tail_.load(std::memory_order_acquire)) {
      return false; // Full
    }

    buffer_[current_head] = std::move(item);
    head_.store(next_head, std::memory_order_release);
    return true;
  }

private:
  // Pad to avoid false sharing
  alignas(CACHE_LINE_SIZE) std::atomic<size_t> head_{0};
  alignas(CACHE_LINE_SIZE) std::atomic<size_t> tail_{0};

  // Data
  std::vector<T, Allocator> buffer_;
  size_t capacity_;
};

} // namespace Core
