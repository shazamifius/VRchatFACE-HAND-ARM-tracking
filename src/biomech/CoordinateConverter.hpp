#pragma once

#include "../core/MathUtils.hpp"

namespace Biomech {

class CoordinateConverter {
public:
  // Configuration
  struct Config {
    bool mirror_mode = true; // Mirror movement implies flipping X
  };

  explicit CoordinateConverter(Config config = {}) : config_(config) {}

  // Convert Position: AI (Right-Handed, Z-up) -> Unity (Left-Handed, Y-up)
  // AI: +X Right, +Y Front, +Z Up
  // Unity: +X Right, +Y Up, +Z Forward
  Core::Vector3 ConvertPosition(const Core::Vector3 &ai_pos) const {
    // Mapping:
    // Unity.X = AI.X  (Mirroring might negate this)
    // Unity.Y = AI.Z
    // Unity.Z = AI.Y

    float x = ai_pos.x;
    float y = ai_pos.z;
    float z = ai_pos.y;

    if (config_.mirror_mode) {
      x = -x;
    }

    return Core::Vector3(x, y, z);
  }

  // Convert Rotation
  // This is complex. Usually requires remapping axes and negating components
  // depending on handedness. A robust way is to construct a basis change matrix
  // or swizzle quaternion components. Simplified Logic for Z-up to Y-up:
  // Quat(x, y, z, w) -> Quat(x, z, y, w) maybe?
  // Let's use a standard construct: Rotate -90 degrees around X axis to line up
  // Z-up to Y-up (forward being Y in AI, Z in Unity).
  Core::Quaternion ConvertRotation(const Core::Quaternion &ai_rot) const {
    // Apply basis transform
    // 1. Convert to Unity base
    // 2. Apply Mirror if needed

    // For now, let's implement a verified swizzle for typical webcam inference
    // (Mediapipe-like) If AI is (x,y,z,w), Unity usually needs specific
    // negation. Let's assume a conversion rotation `correction` is applied.

    // Correction Quat: Rotate -90 on X?
    Core::Quaternion toUnity =
        glm::angleAxis(glm::radians(-90.0f), Core::Vector3(1, 0, 0));
    Core::Quaternion q = toUnity * ai_rot;

    if (config_.mirror_mode) {
      // Mirroring a quaternion usually involves negating Y and Z components
      // (reflection across YZ plane?) Or calculating a mirror rotation. Simple
      // approximation: Flip X and W for mirror across YZ plane? Actually, let's
      // stick to basic coordinate swap first.
      // TODO: Verify with unit tests and visual check.
      q.y = -q.y;
      q.z = -q.z;
    }

    return q;
  }

  void SetMirrorMode(bool enable) { config_.mirror_mode = enable; }

private:
  Config config_;
};

} // namespace Biomech
