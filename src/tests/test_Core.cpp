#include "../core/Profiler.hpp"
#include "../core/RingBuffer.hpp"
#include <gtest/gtest.h>
#include <thread>
#include <vector>

// --- RingBuffer Tests ---

TEST(RingBufferTest, BasicPushPop) {
  Core::RingBuffer<int> rb(4);

  EXPECT_TRUE(rb.empty());

  EXPECT_TRUE(rb.push(1));
  EXPECT_TRUE(rb.push(2));
  EXPECT_FALSE(rb.empty());

  int val;
  EXPECT_TRUE(rb.pop(val));
  EXPECT_EQ(val, 1);

  EXPECT_TRUE(rb.pop(val));
  EXPECT_EQ(val, 2);

  EXPECT_TRUE(rb.empty());
  EXPECT_FALSE(rb.pop(val));
}

TEST(RingBufferTest, FullBuffer) {
  Core::RingBuffer<int> rb(2); // Capacity 2

  EXPECT_TRUE(rb.push(1));
  EXPECT_TRUE(rb.push(2));
  EXPECT_FALSE(rb.push(3)); // Should be full

  int val;
  EXPECT_TRUE(rb.pop(val));
  EXPECT_EQ(val, 1);

  EXPECT_TRUE(rb.push(3)); // Should be able to push now
}

TEST(RingBufferTest, SPSCPushPop) {
  Core::RingBuffer<int> rb(128);
  const int count = 1000;

  std::thread producer([&]() {
    for (int i = 0; i < count; ++i) {
      while (!rb.push(i)) {
        std::this_thread::yield();
      }
    }
  });

  std::thread consumer([&]() {
    int val;
    for (int i = 0; i < count; ++i) {
      while (!rb.pop(val)) {
        std::this_thread::yield();
      }
      EXPECT_EQ(val, i);
    }
  });

  producer.join();
  consumer.join();
}

// --- Profiler Tests ---

TEST(ProfilerTest, BasicRecord) {
  {
    PROFILE_SCOPE("TestScope");
    std::this_thread::sleep_for(std::chrono::milliseconds(1));
  }
  // Just ensure it doesn't crash.
  // Capturing output is harder, but we assume it works if no crash.
  Core::Profiler::Get().Record("TestScope", 100);
}
