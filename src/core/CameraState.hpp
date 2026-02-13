#pragma once

// Simple 3D camera state for visualizer
struct CameraState {
  float distance = 3.5f; // Distance from origin
  float angle_y = 15.0f; // Pitch (rotation around X axis)
  float angle_x = 0.0f;  // Yaw (rotation around Y axis)
  float height = -0.5f;  // Vertical offset

  // Reset to default
  void Reset() {
    distance = 3.5f;
    angle_y = 15.0f;
    angle_x = 0.0f;
    height = -0.5f;
  }
};
