#include "../biomech/CoordinateConverter.hpp"
#include "../biomech/MotionFilter.hpp"
#include "../biomech/SkeletonSolver.hpp"
#include <gtest/gtest.h>

// --- CoordinateConverter Tests ---

TEST(CoordinateConverterTest, BasicConversion) {
  Biomech::CoordinateConverter converter;
  // Test point: (1, 2, 3) in AI (Right-Handed Z-up)
  // Expected Unity: (-1, 3, 2) with Mirror Mode ON (default)
  // Unity X = -AI.X = -1
  // Unity Y = AI.Z = 3
  // Unity Z = AI.Y = 2

  Core::Vector3 ai_pos(1.0f, 2.0f, 3.0f);
  Core::Vector3 unity_pos = converter.ConvertPosition(ai_pos);

  EXPECT_FLOAT_EQ(unity_pos.x, -1.0f);
  EXPECT_FLOAT_EQ(unity_pos.y, 3.0f);
  EXPECT_FLOAT_EQ(unity_pos.z, 2.0f);
}

TEST(CoordinateConverterTest, NoMirror) {
  Biomech::CoordinateConverter converter({false});
  Core::Vector3 ai_pos(1.0f, 2.0f, 3.0f);
  Core::Vector3 unity_pos = converter.ConvertPosition(ai_pos);

  EXPECT_FLOAT_EQ(unity_pos.x, 1.0f); // No flip
  EXPECT_FLOAT_EQ(unity_pos.y, 3.0f);
  EXPECT_FLOAT_EQ(unity_pos.z, 2.0f);
}

// --- MotionFilter Tests ---

TEST(MotionFilterTest, OneEuroStability) {
  Biomech::OneEuroFilter filter;
  Core::Vector3 pos(1.0f, 1.0f, 1.0f);

  // First update initializes
  Core::Vector3 filtered = filter.Filter(pos, 0.0);
  EXPECT_FLOAT_EQ(filtered.x, 1.0f);

  // Second update with same value should stay same
  filtered = filter.Filter(pos, 0.1);
  EXPECT_FLOAT_EQ(filtered.x, 1.0f);
}

TEST(MotionFilterTest, Extrapolation) {
  Core::Vector3 pos(0.0f, 0.0f, 0.0f);
  Core::Vector3 vel(1.0f, 0.0f, 0.0f); // 1 m/s in X

  // Predict 100ms into future
  Core::Vector3 pred = Biomech::Extrapolator::Predict(pos, vel, 100.0f);

  EXPECT_FLOAT_EQ(pred.x, 0.1f); // 0.1m
}

// --- SkeletonSolver Tests ---

TEST(SkeletonSolverTest, BasicSolve) {
  Biomech::SkeletonSolver solver;

  // Construct dummy inputs
  Vision::PoseResult body;
  body.detection_confidence = 0.9f;
  body.keypoints[Vision::PoseResult::NOSE] = {0, 0, 0.9f}; // Valid nose
  body.keypoints[Vision::PoseResult::LEFT_SHOULDER] = {0, 0,
                                                       0.9f}; // Valid Shoulder

  Vision::HandResult leftHand, rightHand;

  auto pose = solver.Solve(body, leftHand, rightHand);

  EXPECT_TRUE(pose.contains(Biomech::HumanBodyBones::Hips));
  EXPECT_FLOAT_EQ(pose[Biomech::HumanBodyBones::Hips].confidence, 0.9f);
}
