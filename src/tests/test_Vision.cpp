#include "../vision/InferenceEngine.hpp"
#include "../vision/ModelSelector.hpp"
#include <gtest/gtest.h>


// --- InferenceEngine Tests ---

TEST(InferenceEngineTest, Instantiation) {
  // Just test that we can create the engine (initializes ONNX Runtime env)
  EXPECT_NO_THROW({ Vision::InferenceEngine engine; });
}

// --- ModelSelector Tests ---

TEST(ModelSelectorTest, InitialState) {
  Vision::ModelSelector selector;
  EXPECT_EQ(selector.GetCurrentMode(), Vision::QualityMode::Unknown);
}

TEST(ModelSelectorTest, UltraDecision) {
  Vision::ModelSelector selector;
  // Feed 15 frames of low latency (10ms = 10000us)
  for (int i = 0; i < 15; ++i) {
    selector.UpdateLatency(10000);
  }

  // Expect Ultra mode (< 12ms)
  EXPECT_EQ(selector.GetCurrentMode(), Vision::QualityMode::Ultra);
  EXPECT_EQ(selector.GetModeString(), "Ultra");
}

TEST(ModelSelectorTest, BalancedDecision) {
  Vision::ModelSelector selector;
  // Feed 15 frames of medium latency (15ms = 15000us)
  for (int i = 0; i < 15; ++i) {
    selector.UpdateLatency(15000);
  }

  // Expect Balanced mode (12-18ms)
  EXPECT_EQ(selector.GetCurrentMode(), Vision::QualityMode::Balanced);
}

TEST(ModelSelectorTest, EcoDecision) {
  Vision::ModelSelector selector;
  // Feed 15 frames of high latency (20ms = 20000us)
  for (int i = 0; i < 15; ++i) {
    selector.UpdateLatency(20000);
  }

  // Expect Eco mode (> 18ms)
  EXPECT_EQ(selector.GetCurrentMode(), Vision::QualityMode::Eco);
}
