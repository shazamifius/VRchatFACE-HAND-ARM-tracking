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

  // Map EAR to blink (inverted: low EAR = closed = high blink value)
  // Typical range: EAR 0.15 (closed) to 0.30 (open)
  // We want: EAR 0.15 → blink=1.0, EAR 0.30 → blink=0.0
  constexpr float EAR_CLOSED = 0.15f;
  constexpr float EAR_OPEN = 0.30f;

  bs.eyeBlinkLeft = 1.0f - MapRange(ear_left, EAR_CLOSED, EAR_OPEN);
  bs.eyeBlinkRight = 1.0f - MapRange(ear_right, EAR_CLOSED, EAR_OPEN);

  // Eye wide (surprise): when EAR is unusually high
  constexpr float EAR_WIDE_THRESHOLD = 0.35f;
  bs.eyeWideLeft = MapRange(ear_left, EAR_OPEN, EAR_WIDE_THRESHOLD);
  bs.eyeWideRight = MapRange(ear_right, EAR_OPEN, EAR_WIDE_THRESHOLD);

  // Eye squint: when eye is partially closed but not blinking
  // Squint occurs around EAR 0.20-0.25
  if (ear_left > EAR_CLOSED && ear_left < EAR_OPEN) {
    bs.eyeSquintLeft = MapRange(ear_left, EAR_OPEN, EAR_CLOSED, 0.0f, 0.6f);
  }
  if (ear_right > EAR_CLOSED && ear_right < EAR_OPEN) {
    bs.eyeSquintRight = MapRange(ear_right, EAR_OPEN, EAR_CLOSED, 0.0f, 0.6f);
  }

  // TODO: Eye look direction (requires iris tracking or separate gaze model)
  // For now, leave eyeLookUp/Down/In/Out at 0.0
}

// === MOUTH BLENDSHAPES ===

float BlendshapeCalculator::CalculateMouthAspectRatio(
    const Vision::FaceMeshResult &face) {
  // Mouth Aspect Ratio (MAR)
  // Similar to EAR but for mouth opening

  using Vision::LandmarkUtils::Distance2D;

  const auto &upper = face.MouthUpperLipTop();
  const auto &lower = face.MouthLowerLipBottom();
  const auto &left_corner = face.MouthLeftCorner();
  const auto &right_corner = face.MouthRightCorner();

  // Vertical distance (lip separation)
  float vertical = Distance2D(upper, lower);

  // Horizontal distance (mouth width)
  float horizontal = Distance2D(left_corner, right_corner);

  if (horizontal < 0.001f)
    return 0.0f;

  // MAR = vertical / horizontal
  // Closed: MAR ≈ 0.05-0.10
  // Speaking: MAR ≈ 0.15-0.30
  // Wide open: MAR ≈ 0.35-0.50+
  float mar = vertical / horizontal;

  return mar;
}

void BlendshapeCalculator::CalculateMouthBlendshapes(
    const Vision::FaceMeshResult &face, ARKitBlendshapes &bs) {

  float mar = CalculateMouthAspectRatio(face);

  // Jaw open: based on MAR
  // Closed: MAR ≈ 0.05, Open: MAR ≈ 0.40
  constexpr float MAR_CLOSED = 0.05f;
  constexpr float MAR_OPEN = 0.40f;
  bs.jawOpen = MapRange(mar, MAR_CLOSED, MAR_OPEN);

  // Smile detection: mouth corners elevated relative to center
  using Vision::LandmarkUtils::Distance2D;

  const auto &left_corner = face.MouthLeftCorner();
  const auto &right_corner = face.MouthRightCorner();
  const auto &nose_tip = face.NoseTip();
  const auto &chin = face.Chin();

  // Calculate baseline (neutral) Y position for mouth corners
  // Approximate: midway between nose and chin
  float baseline_y = (nose_tip.y + chin.y) * 0.5f;

  // Smile: corners above baseline
  float left_elevation =
      baseline_y - left_corner.y; // Positive = above baseline
  float right_elevation = baseline_y - right_corner.y;

  // Map elevation to smile (typical range: -0.02 to +0.05)
  bs.mouthSmileLeft = MapRange(left_elevation, -0.01f, 0.04f);
  bs.mouthSmileRight = MapRange(right_elevation, -0.01f, 0.04f);

  // Frown: corners below baseline (negative elevation)
  bs.mouthFrownLeft = MapRange(-left_elevation, -0.01f, 0.03f);
  bs.mouthFrownRight = MapRange(-right_elevation, -0.01f, 0.03f);

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

} // namespace Biomech
