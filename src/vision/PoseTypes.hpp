#pragma once

#include <algorithm>
#include <array>


namespace Vision {

// Single keypoint with 2D position and confidence
struct PoseKeypoint {
  float x = 0.0f;
  float y = 0.0f;
  float confidence = 0.0f;

  bool IsValid(float threshold = 0.3f) const { return confidence >= threshold; }
};

// YOLOv8-Pose returns 17 COCO keypoints
struct PoseResult {
  static constexpr int NUM_KEYPOINTS = 17;

  // COCO Keypoint indices
  enum KeypointIndex {
    NOSE = 0,
    LEFT_EYE = 1,
    RIGHT_EYE = 2,
    LEFT_EAR = 3,
    RIGHT_EAR = 4,
    LEFT_SHOULDER = 5,
    RIGHT_SHOULDER = 6,
    LEFT_ELBOW = 7,
    RIGHT_ELBOW = 8,
    LEFT_WRIST = 9,
    RIGHT_WRIST = 10,
    LEFT_HIP = 11,
    RIGHT_HIP = 12,
    LEFT_KNEE = 13,
    RIGHT_KNEE = 14,
    LEFT_ANKLE = 15,
    RIGHT_ANKLE = 16
  };

  std::array<PoseKeypoint, NUM_KEYPOINTS> keypoints;
  float detection_confidence = 0.0f;

  // Helper accessors
  const PoseKeypoint &Nose() const { return keypoints[NOSE]; }
  const PoseKeypoint &LeftEye() const { return keypoints[LEFT_EYE]; }
  const PoseKeypoint &RightEye() const { return keypoints[RIGHT_EYE]; }
  const PoseKeypoint &LeftShoulder() const { return keypoints[LEFT_SHOULDER]; }
  const PoseKeypoint &RightShoulder() const {
    return keypoints[RIGHT_SHOULDER];
  }
  const PoseKeypoint &LeftElbow() const { return keypoints[LEFT_ELBOW]; }
  const PoseKeypoint &RightElbow() const { return keypoints[RIGHT_ELBOW]; }
  const PoseKeypoint &LeftWrist() const { return keypoints[LEFT_WRIST]; }
  const PoseKeypoint &RightWrist() const { return keypoints[RIGHT_WRIST]; }
  const PoseKeypoint &LeftHip() const { return keypoints[LEFT_HIP]; }
  const PoseKeypoint &RightHip() const { return keypoints[RIGHT_HIP]; }
  const PoseKeypoint &LeftKnee() const { return keypoints[LEFT_KNEE]; }
  const PoseKeypoint &RightKnee() const { return keypoints[RIGHT_KNEE]; }
  const PoseKeypoint &LeftAnkle() const { return keypoints[LEFT_ANKLE]; }
  const PoseKeypoint &RightAnkle() const { return keypoints[RIGHT_ANKLE]; }

  // Check if pose is valid (at least key points detected)
  bool IsValid() const {
    return detection_confidence > 0.3f && Nose().IsValid() &&
           (LeftShoulder().IsValid() || RightShoulder().IsValid());
  }

  // Get center of shoulders (torso position)
  PoseKeypoint GetTorsoCenter() const {
    PoseKeypoint center;
    if (LeftShoulder().IsValid() && RightShoulder().IsValid()) {
      center.x = (LeftShoulder().x + RightShoulder().x) * 0.5f;
      center.y = (LeftShoulder().y + RightShoulder().y) * 0.5f;
      center.confidence =
          std::min(LeftShoulder().confidence, RightShoulder().confidence);
    } else if (LeftShoulder().IsValid()) {
      center = LeftShoulder();
    } else if (RightShoulder().IsValid()) {
      center = RightShoulder();
    }
    return center;
  }

  // Get center of hips
  PoseKeypoint GetHipsCenter() const {
    PoseKeypoint center;
    if (LeftHip().IsValid() && RightHip().IsValid()) {
      center.x = (LeftHip().x + RightHip().x) * 0.5f;
      center.y = (LeftHip().y + RightHip().y) * 0.5f;
      center.confidence = std::min(LeftHip().confidence, RightHip().confidence);
    } else if (LeftHip().IsValid()) {
      center = LeftHip();
    } else if (RightHip().IsValid()) {
      center = RightHip();
    }
    return center;
  }
};

} // namespace Vision
