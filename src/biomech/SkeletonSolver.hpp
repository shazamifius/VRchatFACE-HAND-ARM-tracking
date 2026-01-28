#pragma once

#include "../core/MathUtils.hpp"
#include <map>
#include <optional>
#include <vector>


namespace Biomech {

// Unity HumanBodyBones enum mapping
enum class HumanBodyBones {
  Hips = 0,
  Spine = 1,
  Chest = 2,
  Neck = 3,
  Head = 4,
  LeftShoulder = 11,
  LeftUpperArm = 13,
  LeftLowerArm = 15,
  LeftHand = 17,
  RightShoulder = 12,
  RightUpperArm = 14,
  RightLowerArm = 16,
  RightHand = 18,
  // ... (Simplified for this plan)
};

struct BoneData {
  Core::Vector3 position;
  Core::Quaternion rotation;
  float confidence;
};

class SkeletonSolver {
public:
  // Maps tracking data to VRChat skeleton
  // This is where we would implement IK or FK propagation

  using SkeletonPose = std::map<HumanBodyBones, BoneData>;

  SkeletonPose Solve(const std::vector<Core::Vector3> &keypoints,
                     const std::vector<float> &confidences) {
    SkeletonPose pose;

    // Example: Map Hip (Keypoint 0 usually in COCO/Body25)
    // This is just a placeholder logic. Real solver requires full IK.
    if (!keypoints.empty()) {
      pose[HumanBodyBones::Hips] = {keypoints[0], Core::Quaternion(1, 0, 0, 0),
                                    confidences[0]};
    }

    return pose;
  }
};

} // namespace Biomech
