#pragma once

#include <cmath>
#include <glm/glm.hpp>
#include <glm/gtc/quaternion.hpp>
#include <glm/gtx/quaternion.hpp>


namespace Core {

// Common types
using Vector3 = glm::vec3;
using Quaternion = glm::quat;
using Matrix4 = glm::mat4;

class MathUtils {
public:
  static constexpr float PI = 3.14159265359f;
  static constexpr float DEG_TO_RAD = PI / 180.0f;
  static constexpr float RAD_TO_DEG = 180.0f / PI;

  static Vector3 Lerp(const Vector3 &a, const Vector3 &b, float t) {
    return glm::mix(a, b, t);
  }

  static Quaternion Slerp(const Quaternion &a, const Quaternion &b, float t) {
    return glm::slerp(a, b, t);
  }
};

} // namespace Core
