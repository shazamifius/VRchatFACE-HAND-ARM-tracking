#include "biomech/BlendshapeTypes.hpp"
#include <cmath>

namespace Biomech {

// Main processing function
ARKitBlendshapes
BlendshapeCalculator::Calculate(const Vision::FaceMeshResult &face) {
  ARKitBlendshapes bs;
  bs.Reset();

  if (!face.IsValid()) {
    return bs; // Return neutral if no valid face
  }

  // Calculate each blendshape category
  CalculateEyeBlendshapes(face, bs);
  CalculateMouthBlendshapes(face, bs);
  CalculateBrowBlendshapes(face, bs);

  return bs;
}

// === EYE BLENDSHAPES ===

float BlendshapeCalculator::CalculateEyeAspectRatio(
    const std::array<Vision::FaceLandmark, 6> &eye) {
  // Eye Aspect Ratio (EAR) algorithm
  // Based on: "Real-Time Eye Blink Detection using Facial Landmarks" (Soukupová
  // & Čech, 2016)
  //
  // eye[0] = left corner
  // eye[1] = top-left
  // eye[2] = top
  // eye[3] = top-right
  // eye[4] = right corner
  // eye[5] = bottom

  using Vision::LandmarkUtils::Distance2D;

  // Vertical distances
  float v1 = Distance2D(eye[2], eye[5]); // Top to bottom

  // Horizontal distance
  float h = Distance2D(eye[0], eye[4]); // Left corner to right corner

  if (h < 0.001f)
    return 0.0f; // Avoid division by zero

  // EAR = vertical / horizontal
  // Open eye: EAR ≈ 0.25-0.35
  // Closed eye: EAR ≈ 0.10-0.15
  float ear = v1 / h;

  return ear;
}

void BlendshapeCalculator::CalculateEyeBlendshapes(
    const Vision::FaceMeshResult &face, ARKitBlendshapes &bs) {

  // Calculate EAR for both eyes
  auto left_eye = face.GetLeftEyeLandmarks();
  auto right_eye = face.GetRightEyeLandmarks();

  float ear_left = CalculateEyeAspectRatio(left_eye);
  float ear_right = CalculateEyeAspectRatio(right_eye);

  // Map EAR to blink using Calibration
  // Inverted: low EAR = closed = high blink value
  bs.eyeBlinkLeft =
      1.0f - MapRange(ear_left, calibration_.ear_closed, calibration_.ear_open);
  bs.eyeBlinkRight = 1.0f - MapRange(ear_right, calibration_.ear_closed,
                                     calibration_.ear_open);

  // --- SMART BLINK SNAP (User Request) ---
  // If blink is detected (mostly closed), force it to fully closed (1.0)
  // This avoids "half-lidded" or vibrating eyes during fast blinks.
  constexpr float BLINK_SNAP_THRESHOLD = 0.8f;
  if (bs.eyeBlinkLeft > BLINK_SNAP_THRESHOLD)
    bs.eyeBlinkLeft = 1.0f;
  if (bs.eyeBlinkRight > BLINK_SNAP_THRESHOLD)
    bs.eyeBlinkRight = 1.0f;

  // Eye wide (surprise): EAR > Normal
  bs.eyeWideLeft =
      MapRange(ear_left, calibration_.ear_open, calibration_.ear_wide);
  bs.eyeWideRight =
      MapRange(ear_right, calibration_.ear_open, calibration_.ear_wide);

  // Eye squint: Partial close
  if (ear_left > calibration_.ear_closed && ear_left < calibration_.ear_open) {
    bs.eyeSquintLeft = MapRange(ear_left, calibration_.ear_open,
                                calibration_.ear_closed, 0.0f, 0.8f);
  }
  if (ear_right > calibration_.ear_closed &&
      ear_right < calibration_.ear_open) {
    bs.eyeSquintRight = MapRange(ear_right, calibration_.ear_open,
                                 calibration_.ear_closed, 0.0f, 0.8f);
  }
}

// === MOUTH BLENDSHAPES ===

float BlendshapeCalculator::CalculateMouthAspectRatio(
    const Vision::FaceMeshResult &face) {
  // Mouth Aspect Ratio (MAR)
  using Vision::LandmarkUtils::Distance2D;

  const auto &upper = face.MouthUpperLipTop();
  const auto &lower = face.MouthLowerLipBottom();
  const auto &left_corner = face.MouthLeftCorner();
  const auto &right_corner = face.MouthRightCorner();

  float vertical = Distance2D(upper, lower);
  float horizontal = Distance2D(left_corner, right_corner);

  if (horizontal < 0.001f)
    return 0.0f;

  return vertical / horizontal;
}

void BlendshapeCalculator::CalculateMouthBlendshapes(
    const Vision::FaceMeshResult &face, ARKitBlendshapes &bs) {

  float mar = CalculateMouthAspectRatio(face);

  // Jaw open: based on MAR
  // Adjusted: More sensitive (0.40 -> 0.25)
  constexpr float MAR_CLOSED = 0.02f;
  constexpr float MAR_OPEN = 0.25f; // User reported "bas sa ouvre pas"
  bs.jawOpen = MapRange(mar, MAR_CLOSED, MAR_OPEN);

  // Smile detection: Updated logic (Reference to Lip Center instead of
  // Chin/Nose)
  using Vision::LandmarkUtils::Distance2D;

  const auto &left_corner = face.MouthLeftCorner();
  const auto &right_corner = face.MouthRightCorner();
  const auto &upper_lip = face.MouthUpperLipTop();
  const auto &lower_lip = face.MouthLowerLipBottom();

  // Calculate Mouth Center Y
  float mouth_center_y = (upper_lip.y + lower_lip.y) * 0.5f;

  // Smile: Corners ABOVE center (Y is smaller)
  float left_elevation = mouth_center_y - left_corner.y;
  float right_elevation = mouth_center_y - right_corner.y;

  // Sensitive smile thresholds
  // Sensitive smile thresholds
  float SMILE_THRESH_MIN = 0.005f; // Slight lift
  float SMILE_THRESH_MAX = calibration_.smile_elevation_max;

  bs.mouthSmileLeft =
      MapRange(left_elevation, SMILE_THRESH_MIN, SMILE_THRESH_MAX);
  bs.mouthSmileRight =
      MapRange(right_elevation, SMILE_THRESH_MIN, SMILE_THRESH_MAX);

  // FIX: Suppress smile when jaw is open (prevents "Fake Smile" on open mouth)
  // When jaw opens, corners naturally move differently. We subtract jaw
  // influence.
  float suppression = bs.jawOpen * 0.4f;
  bs.mouthSmileLeft = Clamp(bs.mouthSmileLeft - suppression, 0.0f, 1.0f);
  bs.mouthSmileRight = Clamp(bs.mouthSmileRight - suppression, 0.0f, 1.0f);

  // Apply Deadzones (Stabilize Neutral)
  bs.mouthSmileLeft = ApplyDeadzone(bs.mouthSmileLeft, 0.05f);
  bs.mouthSmileRight = ApplyDeadzone(bs.mouthSmileRight, 0.05f);

  // Frown: Corners BELOW center (Y is larger -> elevation negative)
  // Adjusted sensitivity
  bs.mouthFrownLeft = MapRange(-left_elevation, SMILE_THRESH_MIN, 0.03f);
  bs.mouthFrownRight = MapRange(-right_elevation, SMILE_THRESH_MIN, 0.03f);

  bs.mouthFrownLeft = ApplyDeadzone(bs.mouthFrownLeft, 0.05f);
  bs.mouthFrownRight = ApplyDeadzone(bs.mouthFrownRight, 0.05f);

  // Mouth funnel (lips forward, like "ooo")
  // Approximation: narrow mouth width with some opening
  const auto &nose_bridge = face.NoseBridge();
  float mouth_width = Distance2D(left_corner, right_corner);
  float face_width_proxy = Distance2D(face.LeftCheek(), face.RightCheek());

  if (face_width_proxy > 0.001f) {
    float width_ratio = mouth_width / face_width_proxy;
    // Normal: ≈0.5, Funneled: ≈0.3
    if (width_ratio < 0.45f && mar > 0.1f) {
      bs.mouthFunnel = MapRange(0.45f - width_ratio, 0.0f, 0.15f);
    }
  }

  // Mouth pucker (kiss shape): narrow + slightly forward
  // Similar to funnel but with lower MAR
  if (mouth_width < 0.001f) {
    bs.mouthPucker = 0.0f;
  } else {
    float aspect = mar / (mouth_width + 0.001f);
    if (aspect < 0.2f) {
      bs.mouthPucker = MapRange(0.2f - aspect, 0.0f, 0.15f);
    }
  }

  // TODO: More advanced mouth shapes (stretch, roll, shrug, press, dimple)
  // These require analyzing inner lip contours
}

// === BROW BLENDSHAPES ===

void BlendshapeCalculator::CalculateBrowBlendshapes(
    const Vision::FaceMeshResult &face, ARKitBlendshapes &bs) {

  using Vision::LandmarkUtils::Distance2D;

  const auto &left_brow_inner = face.LeftBrowInner();
  const auto &left_brow_outer = face.LeftBrowOuter();
  const auto &right_brow_inner = face.RightBrowInner();
  const auto &right_brow_outer = face.RightBrowOuter();
  const auto &nose_bridge = face.NoseBridge();

  // Calculate baseline (neutral) Y position for brows
  // Use nose bridge as reference
  float baseline_y =
      nose_bridge.y - 0.08f; // Brows typically ~8% above nose bridge

  // Inner brow up (worried/sad expression)
  float left_inner_elevation = baseline_y - left_brow_inner.y;
  float right_inner_elevation = baseline_y - right_brow_inner.y;

  // Average both inner brows
  float inner_avg = (left_inner_elevation + right_inner_elevation) * 0.5f;
  bs.browInnerUp = MapRange(inner_avg, -0.01f, 0.03f);

  // Outer brow up (surprise)
  float left_outer_elevation = baseline_y - left_brow_outer.y;
  float right_outer_elevation = baseline_y - right_brow_outer.y;

  bs.browOuterUpLeft = MapRange(left_outer_elevation, -0.01f, 0.04f);
  bs.browOuterUpRight = MapRange(right_outer_elevation, -0.01f, 0.04f);

  // Brow down (angry/concentrated)
  // Negative elevation = brow lowered
  bs.browDownLeft = MapRange(-left_inner_elevation, -0.01f, 0.025f);
  bs.browDownRight = MapRange(-right_inner_elevation, -0.01f, 0.025f);

  // TODO: Cheek and nose blendshapes (require more landmark analysis)
}

// === AUTO-TUNING ===
void BlendshapeCalculator::PerformAutoTuning(
    const Vision::FaceMeshResult &face) {
  if (!face.IsValid())
    return;

  // 1. Calculate Eye Aspect Ratio (Maximum of both)
  auto left_eye = face.GetLeftEyeLandmarks();
  auto right_eye = face.GetRightEyeLandmarks();
  float ear_left = CalculateEyeAspectRatio(left_eye);
  float ear_right = CalculateEyeAspectRatio(right_eye);
  float max_ear = std::max(ear_left, ear_right);

  // 2. Calculate Mouth Width (Not currently used in AutoUpdate but useful for
  // future) Reuse CalculateMouthAspectRatio or just dist float mar =
  // CalculateMouthAspectRatio(face); float mouth_width = ...

  // 3. Calculate Brow Elevation (vs Nose)
  const auto &nose_bridge = face.NoseBridge();
  // We want MAX elevation (Surprise)
  // Lower Y = Higher Brow
  float min_brow_y =
      std::min({face.LeftBrowInner().y, face.LeftBrowOuter().y,
                face.RightBrowInner().y, face.RightBrowOuter().y});
  // Elevation = Neutral - Current. But here we just pass raw Y to calibration?
  // UserCalibration::UpdateAuto logic needs to be clear on what it takes.
  // In UserCalibration.hpp we wrote: UpdateAuto(current_ear,
  // current_mouth_width, current_brow_elev)

  // Checking UserCalibration.hpp again:
  // "if (current_ear > ear_wide)"

  // So we pass the "Wide" metric.

  // For brows, we didn't fully implement brow auto-tuning in header yet, but
  // passed it. Let's pass 0.0f for now for mouth/brows if not ready, or
  // implement basic.

  // Calling the calibration update
  calibration_.UpdateAuto(max_ear, 0.0f, 0.0f);
}

} // namespace Biomech
