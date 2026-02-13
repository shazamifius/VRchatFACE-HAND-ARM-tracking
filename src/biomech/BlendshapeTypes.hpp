#pragma once

#include "biomech/UserCalibration.hpp"
#include "vision/FaceTypes.hpp"
#include <cmath>

namespace Biomech {

// ARKit standard blendshapes (VRCFaceTracking v2 compatible)
// Reference:
// https://developer.apple.com/documentation/arkit/arfaceanchor/blendshapelocation
struct ARKitBlendshapes {
  // === EYES === (10 parameters)
  float eyeBlinkLeft = 0.0f;     // Left eye closed [0=open, 1=closed]
  float eyeBlinkRight = 0.0f;    // Right eye closed
  float eyeLookUpLeft = 0.0f;    // Left eye looking up
  float eyeLookUpRight = 0.0f;   // Right eye looking up
  float eyeLookDownLeft = 0.0f;  // Left eye looking down
  float eyeLookDownRight = 0.0f; // Right eye looking down
  float eyeLookInLeft = 0.0f;    // Left eye looking inward (toward nose)
  float eyeLookInRight = 0.0f;   // Right eye looking inward
  float eyeLookOutLeft = 0.0f;   // Left eye looking outward
  float eyeLookOutRight = 0.0f;  // Right eye looking outward

  // Eye additional
  float eyeSquintLeft = 0.0f;  // Left eye squinting
  float eyeSquintRight = 0.0f; // Right eye squinting
  float eyeWideLeft = 0.0f;    // Left eye wide open (surprise)
  float eyeWideRight = 0.0f;   // Right eye wide open

  // === JAW === (4 parameters)
  float jawOpen = 0.0f;    // Jaw opening [0=closed, 1=wide open]
  float jawForward = 0.0f; // Jaw pushed forward
  float jawLeft = 0.0f;    // Jaw moved to left
  float jawRight = 0.0f;   // Jaw moved to right

  // === MOUTH === (20 parameters)
  float mouthClose = 0.0f;          // Lips pressed together
  float mouthFunnel = 0.0f;         // Lips funneled (like "ooo")
  float mouthPucker = 0.0f;         // Lips puckered (kiss)
  float mouthLeft = 0.0f;           // Mouth moved to left
  float mouthRight = 0.0f;          // Mouth moved to right
  float mouthSmileLeft = 0.0f;      // Left corner smile
  float mouthSmileRight = 0.0f;     // Right corner smile
  float mouthFrownLeft = 0.0f;      // Left corner frown
  float mouthFrownRight = 0.0f;     // Right corner frown
  float mouthDimpleLeft = 0.0f;     // Left dimple
  float mouthDimpleRight = 0.0f;    // Right dimple
  float mouthStretchLeft = 0.0f;    // Left corner stretched
  float mouthStretchRight = 0.0f;   // Right corner stretched
  float mouthRollLower = 0.0f;      // Lower lip rolled in
  float mouthRollUpper = 0.0f;      // Upper lip rolled in
  float mouthShrugLower = 0.0f;     // Lower lip shrug
  float mouthShrugUpper = 0.0f;     // Upper lip shrug
  float mouthPressLeft = 0.0f;      // Left lip pressed
  float mouthPressRight = 0.0f;     // Right lip pressed
  float mouthLowerDownLeft = 0.0f;  // Lower lip pulled down left
  float mouthLowerDownRight = 0.0f; // Lower lip pulled down right
  float mouthUpperUpLeft = 0.0f;    // Upper lip pulled up left
  float mouthUpperUpRight = 0.0f;   // Upper lip pulled up right

  // === BROWS === (5 parameters)
  float browDownLeft = 0.0f;     // Left brow lowered (angry)
  float browDownRight = 0.0f;    // Right brow lowered
  float browInnerUp = 0.0f;      // Inner brows raised (sad/worried)
  float browOuterUpLeft = 0.0f;  // Left outer brow raised (surprise)
  float browOuterUpRight = 0.0f; // Right outer brow raised

  // === CHEEKS === (3 parameters)
  float cheekPuff = 0.0f;        // Cheeks puffed out
  float cheekSquintLeft = 0.0f;  // Left cheek squint
  float cheekSquintRight = 0.0f; // Right cheek squint

  // === NOSE === (2 parameters)
  float noseSneerLeft = 0.0f;  // Left nose sneer (disgust)
  float noseSneerRight = 0.0f; // Right nose sneer

  // === TONGUE === (1 parameter - optional)
  float tongueOut = 0.0f; // Tongue sticking out

  // === HEAD POSE === (3 parameters)
  float headPitch = 0.0f; // Up/Down (X-axis)
  float headYaw = 0.0f;   // Left/Right (Y-axis)
  float headRoll = 0.0f;  // Tilt (Z-axis)

  // Total: 52 blendshapes (full ARKit set)

  // Helper: Reset all to neutral
  void Reset() {
    eyeBlinkLeft = eyeBlinkRight = 0.0f;
    eyeLookUpLeft = eyeLookUpRight = 0.0f;
    eyeLookDownLeft = eyeLookDownRight = 0.0f;
    eyeLookInLeft = eyeLookInRight = 0.0f;
    eyeLookOutLeft = eyeLookOutRight = 0.0f;
    eyeSquintLeft = eyeSquintRight = 0.0f;
    eyeWideLeft = eyeWideRight = 0.0f;

    jawOpen = jawForward = jawLeft = jawRight = 0.0f;

    mouthClose = mouthFunnel = mouthPucker = 0.0f;
    mouthLeft = mouthRight = 0.0f;
    mouthSmileLeft = mouthSmileRight = 0.0f;
    mouthFrownLeft = mouthFrownRight = 0.0f;
    mouthDimpleLeft = mouthDimpleRight = 0.0f;
    mouthStretchLeft = mouthStretchRight = 0.0f;
    mouthRollLower = mouthRollUpper = 0.0f;
    mouthShrugLower = mouthShrugUpper = 0.0f;
    mouthPressLeft = mouthPressRight = 0.0f;
    mouthLowerDownLeft = mouthLowerDownRight = 0.0f;
    mouthUpperUpLeft = mouthUpperUpRight = 0.0f;

    browDownLeft = browDownRight = 0.0f;
    browInnerUp = browOuterUpLeft = browOuterUpRight = 0.0f;

    cheekPuff = cheekSquintLeft = cheekSquintRight = 0.0f;

    noseSneerLeft = noseSneerRight = 0.0f;

    tongueOut = 0.0f;

    headPitch = headYaw = headRoll = 0.0f;
  }

  // Define JSON serialization for all 52 fields
  NLOHMANN_DEFINE_TYPE_INTRUSIVE(
      ARKitBlendshapes, eyeBlinkLeft, eyeBlinkRight, eyeLookUpLeft,
      eyeLookUpRight, eyeLookDownLeft, eyeLookDownRight, eyeLookInLeft,
      eyeLookInRight, eyeLookOutLeft, eyeLookOutRight, eyeSquintLeft,
      eyeSquintRight, eyeWideLeft, eyeWideRight, jawOpen, jawForward, jawLeft,
      jawRight, mouthClose, mouthFunnel, mouthPucker, mouthLeft, mouthRight,
      mouthSmileLeft, mouthSmileRight, mouthFrownLeft, mouthFrownRight,
      mouthDimpleLeft, mouthDimpleRight, mouthStretchLeft, mouthStretchRight,
      mouthRollLower, mouthRollUpper, mouthShrugLower, mouthShrugUpper,
      mouthPressLeft, mouthPressRight, mouthLowerDownLeft, mouthLowerDownRight,
      mouthUpperUpLeft, mouthUpperUpRight, browDownLeft, browDownRight,
      browInnerUp, browOuterUpLeft, browOuterUpRight, cheekPuff,
      cheekSquintLeft, cheekSquintRight, noseSneerLeft, noseSneerRight,
      tongueOut, headPitch, headYaw, headRoll)
};

// Blendshape calculator: 468 landmarks → ARKit blendshapes
class BlendshapeCalculator {
public:
  BlendshapeCalculator() = default;

  // Main computation function
  ARKitBlendshapes Calculate(const Vision::FaceMeshResult &face);

  // Auto-Tuning (Continuous Learning)
  void PerformAutoTuning(const Vision::FaceMeshResult &face);

private:
  // === Eye calculations ===
  float CalculateEyeAspectRatio(
      const std::array<Vision::FaceLandmark, 6> &eye_landmarks);
  void CalculateEyeBlendshapes(const Vision::FaceMeshResult &face,
                               ARKitBlendshapes &bs);

  // === Mouth calculations ===
  float CalculateMouthAspectRatio(const Vision::FaceMeshResult &face);
  void CalculateMouthBlendshapes(const Vision::FaceMeshResult &face,
                                 ARKitBlendshapes &bs);

  // === Brow calculations ===
  void CalculateBrowBlendshapes(const Vision::FaceMeshResult &face,
                                ARKitBlendshapes &bs);

  // Helper utilities
  float Clamp(float value, float min = 0.0f, float max = 1.0f) const {
    return Vision::LandmarkUtils::Clamp(value, min, max);
  }

  float MapRange(float value, float in_min, float in_max, float out_min = 0.0f,
                 float out_max = 1.0f) const {
    return Vision::LandmarkUtils::MapRange(value, in_min, in_max, out_min,
                                           out_max);
  }

  // New: Deadzone to stabilize "Neutral" face
  float ApplyDeadzone(float value, float threshold) const {
    if (value < threshold)
      return 0.0f;
    return value;
  }

  // --- CALIBRATION ---
public:
  void SetCalibration(const UserCalibration &cal) { calibration_ = cal; }
  UserCalibration &GetCalibration() { return calibration_; }

private:
  UserCalibration calibration_;
};

} // namespace Biomech
