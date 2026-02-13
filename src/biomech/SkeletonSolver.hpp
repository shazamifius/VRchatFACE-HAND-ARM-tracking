#pragma once

#include "../core/MathUtils.hpp"
#include "../vision/HandTracking.hpp"
#include "../vision/PoseTypes.hpp"
#include <map>
#include <optional>
#include <vector>


namespace Biomech {
// Unity HumanBodyBones enum mapping (Complete)
enum class HumanBodyBones {
  Hips = 0,
  LeftUpperLeg = 1,
  RightUpperLeg = 2,
  LeftLowerLeg = 3,
  RightLowerLeg = 4,
  LeftFoot = 5,
  RightFoot = 6,
  Spine = 7,
  Chest = 8,
  Neck = 9,
  Head = 10,
  LeftShoulder = 11,
  RightShoulder = 12,
  LeftUpperArm = 13,
  RightUpperArm = 14,
  LeftLowerArm = 15,
  RightLowerArm = 16,
  LeftHand = 17,
  RightHand = 18,
  LeftToes = 19,
  RightToes = 20,
  LeftEye = 21,
  RightEye = 22,
  Jaw = 23,
  LeftThumbProximal = 24,
  LeftThumbIntermediate = 25,
  LeftThumbDistal = 26,
  LeftIndexProximal = 27,
  LeftIndexIntermediate = 28,
  LeftIndexDistal = 29,
  LeftMiddleProximal = 30,
  LeftMiddleIntermediate = 31,
  LeftMiddleDistal = 32,
  LeftRingProximal = 33,
  LeftRingIntermediate = 34,
  LeftRingDistal = 35,
  LeftLittleProximal = 36,
  LeftLittleIntermediate = 37,
  LeftLittleDistal = 38,
  RightThumbProximal = 39,
  RightThumbIntermediate = 40,
  RightThumbDistal = 41,
  RightIndexProximal = 42,
  RightIndexIntermediate = 43,
  RightIndexDistal = 44,
  RightMiddleProximal = 45,
  RightMiddleIntermediate = 46,
  RightMiddleDistal = 47,
  RightRingProximal = 48,
  RightRingIntermediate = 49,
  RightRingDistal = 50,
  RightLittleProximal = 51,
  RightLittleIntermediate = 52,
  RightLittleDistal = 53,
  UpperChest = 54
};

struct BoneData {
  Core::Vector3 position;
  Core::Quaternion rotation;
  float confidence;
};

class SkeletonSolver {
public:
  using SkeletonPose = std::map<HumanBodyBones, BoneData>;

  // Helpers to clear pose
  SkeletonPose CreateEmptyPose() { return SkeletonPose(); }

  SkeletonPose Solve(const Vision::PoseResult &body,
                     const Vision::HandResult &leftHand,
                     const Vision::HandResult &rightHand) {
    SkeletonPose pose;

    // 1. Solve Body (Very Basic FK for now)
    if (body.IsValid()) {
      // Hips center
      auto hips = body.GetHipsCenter();
      // Invert X for mirroring done in CoordinateConverter later, or handle
      // here? Currently CoordinateConverter handles position. We pass raw
      // normalized coords mostly? No, usually we want local rotations.

      // For VRChat, we mostly care about ROTATIONS.
      // Positions are usually only for Hips (Root).

      pose[HumanBodyBones::Hips] = {
          Core::Vector3(hips.x, hips.y, 0), // Z is 0 for 2D body
          Core::Quaternion(1, 0, 0, 0), hips.confidence};

      // Shoulders/Arms
      // Implementation of simple 'LookAt' rotations for arms
      // Right Arm
      if (body.RightShoulder().IsValid() && body.RightElbow().IsValid()) {
        Core::Vector3 dir = {body.RightElbow().x - body.RightShoulder().x,
                             body.RightElbow().y - body.RightShoulder().y, 0};
        // Normalize
        float len = sqrt(dir.x * dir.x + dir.y * dir.y);
        // Calculate angle.. this is a placeholder for real IK
      }
    }

    // 2. Solve Hands (Fingers)
    if (leftHand.IsValid()) {
      SolveHand(leftHand, true, pose);
    }
    if (rightHand.IsValid()) {
      SolveHand(rightHand, false, pose);
    }

    return pose;
  }

private:
  void SolveHand(const Vision::HandResult &hand, bool isLeft,
                 SkeletonPose &pose) {
    // MediaPipe Landmarks:
    // 0: Wrist
    // 1-4: Thumb (CMC, MCP, IP, Tip)
    // 5-8: Index (MCP, PIP, DIP, Tip)
    // 9-12: Middle
    // 13-16: Ring
    // 17-20: Pinky

    // Mapping to Unity Bones
    // Unity Thumb: Proximal (1->2), Intermediate (2->3), Distal (3->4)
    // Unity Fingers: Proximal (MCP->PIP), Intermediate (PIP->DIP), Distal
    // (DIP->Tip)

    // Helper for finger processing
    auto ProcessFinger = [&](int startIdx, HumanBodyBones proximal,
                             HumanBodyBones intermediate,
                             HumanBodyBones distal) {
      // Proximal Rotation: Vector from startIdx to startIdx+1
      Core::Vector3 v1 = {
          hand.landmarks[startIdx + 1].x - hand.landmarks[startIdx].x,
          hand.landmarks[startIdx + 1].y - hand.landmarks[startIdx].y,
          hand.landmarks[startIdx + 1].z - hand.landmarks[startIdx].z};
      // Intermediate
      Core::Vector3 v2 = {
          hand.landmarks[startIdx + 2].x - hand.landmarks[startIdx + 1].x,
          hand.landmarks[startIdx + 2].y - hand.landmarks[startIdx + 1].y,
          hand.landmarks[startIdx + 2].z - hand.landmarks[startIdx + 1].z};
      // Distal
      Core::Vector3 v3 = {
          hand.landmarks[startIdx + 3].x - hand.landmarks[startIdx + 2].x,
          hand.landmarks[startIdx + 3].y - hand.landmarks[startIdx + 2].y,
          hand.landmarks[startIdx + 3].z - hand.landmarks[startIdx + 2].z};

      // Convert vectors to quats (Simplified, assuming default OPEN hand is
      // flat) This needs a proper "Rest Pose" comparison for rotations. For
      // now, let's output Identity or specific rotations based on curl

      // Calculating Curl is easier for basic VRChat input
      // But VMC requires Quaternions.

      // Placeholder: Just putting Identity with high confidence to show "Bone
      // Exists" Real implementation needs `LookAt` or Quat between vectors.
      pose[proximal] = {v1, Core::Quaternion(1, 0, 0, 0), 1.0f};
      pose[intermediate] = {v2, Core::Quaternion(1, 0, 0, 0), 1.0f};
      pose[distal] = {v3, Core::Quaternion(1, 0, 0, 0), 1.0f};
    };

    if (isLeft) {
      ProcessFinger(1, HumanBodyBones::LeftThumbProximal,
                    HumanBodyBones::LeftThumbIntermediate,
                    HumanBodyBones::LeftThumbDistal);
      ProcessFinger(5, HumanBodyBones::LeftIndexProximal,
                    HumanBodyBones::LeftIndexIntermediate,
                    HumanBodyBones::LeftIndexDistal);
      ProcessFinger(9, HumanBodyBones::LeftMiddleProximal,
                    HumanBodyBones::LeftMiddleIntermediate,
                    HumanBodyBones::LeftMiddleDistal);
      ProcessFinger(13, HumanBodyBones::LeftRingProximal,
                    HumanBodyBones::LeftRingIntermediate,
                    HumanBodyBones::LeftRingDistal);
      ProcessFinger(17, HumanBodyBones::LeftLittleProximal,
                    HumanBodyBones::LeftLittleIntermediate,
                    HumanBodyBones::LeftLittleDistal);
      // Hand (Wrist)
      pose[HumanBodyBones::LeftHand] = {
          {hand.landmarks[0].x, hand.landmarks[0].y, hand.landmarks[0].z},
          Core::Quaternion(1, 0, 0, 0),
          1.0f};
    } else {
      ProcessFinger(1, HumanBodyBones::RightThumbProximal,
                    HumanBodyBones::RightThumbIntermediate,
                    HumanBodyBones::RightThumbDistal);
      ProcessFinger(5, HumanBodyBones::RightIndexProximal,
                    HumanBodyBones::RightIndexIntermediate,
                    HumanBodyBones::RightIndexDistal);
      ProcessFinger(9, HumanBodyBones::RightMiddleProximal,
                    HumanBodyBones::RightMiddleIntermediate,
                    HumanBodyBones::RightMiddleDistal);
      ProcessFinger(13, HumanBodyBones::RightRingProximal,
                    HumanBodyBones::RightRingIntermediate,
                    HumanBodyBones::RightRingDistal);
      ProcessFinger(17, HumanBodyBones::RightLittleProximal,
                    HumanBodyBones::RightLittleIntermediate,
                    HumanBodyBones::RightLittleDistal);
      pose[HumanBodyBones::RightHand] = {
          {hand.landmarks[0].x, hand.landmarks[0].y, hand.landmarks[0].z},
          Core::Quaternion(1, 0, 0, 0),
          1.0f};
    }
  }
};

} // namespace Biomech
