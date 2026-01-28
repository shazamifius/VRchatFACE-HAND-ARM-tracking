#pragma once

#include <array>
#include <cmath>

namespace Vision {

// Single face landmark with 3D position
struct FaceLandmark {
  float x = 0.0f;          // Normalized X coordinate [0, 1]
  float y = 0.0f;          // Normalized Y coordinate [0, 1]
  float z = 0.0f;          // Depth (relative to face plane)
  float visibility = 1.0f; // Confidence [0, 1]

  bool IsValid(float threshold = 0.5f) const { return visibility >= threshold; }
};

// MediaPipe Face Mesh result with 468 landmarks
struct FaceMeshResult {
  static constexpr int NUM_LANDMARKS = 468;

  std::array<FaceLandmark, NUM_LANDMARKS> landmarks;
  float detection_confidence = 0.0f;

  bool IsValid() const { return detection_confidence > 0.5f; }

  // Key landmark accessors (MediaPipe indices)
  // Reference:
  // https://github.com/google/mediapipe/blob/master/mediapipe/modules/face_geometry/data/canonical_face_model_uv_visualization.png

  // Eyes
  const FaceLandmark &LeftEyeCenter() const {
    return landmarks[468];
  } // Left eye iris center
  const FaceLandmark &RightEyeCenter() const {
    return landmarks[473];
  } // Right eye iris center

  // Left eye contour (6 points for EAR calculation)
  const FaceLandmark &LeftEyeTop() const { return landmarks[159]; }
  const FaceLandmark &LeftEyeBottom() const { return landmarks[145]; }
  const FaceLandmark &LeftEyeLeft() const { return landmarks[33]; }
  const FaceLandmark &LeftEyeRight() const { return landmarks[133]; }

  // Right eye contour
  const FaceLandmark &RightEyeTop() const { return landmarks[386]; }
  const FaceLandmark &RightEyeBottom() const { return landmarks[374]; }
  const FaceLandmark &RightEyeLeft() const { return landmarks[362]; }
  const FaceLandmark &RightEyeRight() const { return landmarks[263]; }

  // Mouth
  const FaceLandmark &MouthUpperLipTop() const { return landmarks[13]; }
  const FaceLandmark &MouthLowerLipBottom() const { return landmarks[14]; }
  const FaceLandmark &MouthLeftCorner() const { return landmarks[61]; }
  const FaceLandmark &MouthRightCorner() const { return landmarks[291]; }

  // Nose
  const FaceLandmark &NoseTip() const { return landmarks[1]; }
  const FaceLandmark &NoseBridge() const { return landmarks[168]; }

  // Eyebrows
  const FaceLandmark &LeftBrowInner() const { return landmarks[55]; }
  const FaceLandmark &LeftBrowOuter() const { return landmarks[46]; }
  const FaceLandmark &RightBrowInner() const { return landmarks[285]; }
  const FaceLandmark &RightBrowOuter() const { return landmarks[276]; }

  // Face outline
  const FaceLandmark &Chin() const { return landmarks[152]; }
  const FaceLandmark &LeftCheek() const { return landmarks[234]; }
  const FaceLandmark &RightCheek() const { return landmarks[454]; }

  // Helper: Get array of landmarks for left eye (all 6 points)
  std::array<FaceLandmark, 6> GetLeftEyeLandmarks() const {
    return {
        landmarks[33],  // Left corner
        landmarks[160], // Top-left
        landmarks[159], // Top
        landmarks[158], // Top-right
        landmarks[133], // Right corner
        landmarks[153], // Bottom-left
    };
  }

  // Helper: Get array of landmarks for right eye
  std::array<FaceLandmark, 6> GetRightEyeLandmarks() const {
    return {
        landmarks[362], // Left corner
        landmarks[385], // Top-left
        landmarks[386], // Top
        landmarks[387], // Top-right
        landmarks[263], // Right corner
        landmarks[380], // Bottom-left
    };
  }
};

// Helper functions for landmark calculations
namespace LandmarkUtils {

// Calculate Euclidean distance between two landmarks
inline float Distance(const FaceLandmark &a, const FaceLandmark &b) {
  float dx = a.x - b.x;
  float dy = a.y - b.y;
  float dz = a.z - b.z;
  return std::sqrt(dx * dx + dy * dy + dz * dz);
}

// Calculate 2D distance (ignoring Z)
inline float Distance2D(const FaceLandmark &a, const FaceLandmark &b) {
  float dx = a.x - b.x;
  float dy = a.y - b.y;
  return std::sqrt(dx * dx + dy * dy);
}

// Clamp value to range [min, max]
inline float Clamp(float value, float min, float max) {
  if (value < min)
    return min;
  if (value > max)
    return max;
  return value;
}

// Linear interpolation
inline float Lerp(float a, float b, float t) { return a + (b - a) * t; }

// Map value from one range to another
inline float MapRange(float value, float in_min, float in_max, float out_min,
                      float out_max) {
  float normalized = (value - in_min) / (in_max - in_min);
  return Lerp(out_min, out_max, Clamp(normalized, 0.0f, 1.0f));
}

} // namespace LandmarkUtils

} // namespace Vision
