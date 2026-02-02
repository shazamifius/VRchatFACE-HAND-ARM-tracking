#pragma once

#include "vision/FaceTypes.hpp"
#include <algorithm>
#include <cmath>
#include <fstream>
#include <iostream>
#include <nlohmann/json.hpp>

namespace Biomech {

enum class CalibrationCommand {
  NONE,
  NEUTRAL,
  SMILE,
  BROWS_UP,
  BLINK_CLOSED,
  SAVE
};

struct UserCalibration {
  // --- SETTINGS ---
  // Eyes
  float ear_closed = 0.15f; // Eyes closed
  float ear_open = 0.30f;   // Eyes normal open
  float ear_wide = 0.35f;   // Eyes wide (surprise)

  // Mouth
  float mouth_width_neutral = 0.0f; // Will be set on first frame
  float mouth_width_smile_max = 0.0f;
  float smile_elevation_max = 0.035f; // Added for Smile Calibration

  // Brows
  float brow_y_neutral = 0.0f;
  float brow_y_up_max = 0.0f;
  float brow_y_down_max = 0.0f;

  // --- STATE ---
  bool is_initialized = false;

  // --- JSON SERIALIZATION ---
  NLOHMANN_DEFINE_TYPE_INTRUSIVE(UserCalibration, ear_closed, ear_open,
                                 ear_wide, mouth_width_neutral,
                                 mouth_width_smile_max, smile_elevation_max,
                                 brow_y_neutral, brow_y_up_max, brow_y_down_max,
                                 is_initialized)

  // --- METHODS ---

  void Save(const std::string &path) {
    try {
      nlohmann::json j = *this;
      std::ofstream file(path);
      file << j.dump(4);
      std::cout << "[Calibration] Saved to " << path << std::endl;
    } catch (const std::exception &e) {
      std::cerr << "[Calibration] Error saving: " << e.what() << std::endl;
    }
  }

  bool Load(const std::string &path) {
    try {
      std::ifstream file(path);
      if (!file.is_open())
        return false;
      nlohmann::json j;
      file >> j;
      *this = j.get<UserCalibration>();
      std::cout << "[Calibration] Loaded profile from " << path << std::endl;
      return true;
    } catch (const std::exception &e) {
      std::cerr << "[Calibration] Error loading: " << e.what() << std::endl;
      return false;
    }
  }

  // Capture precise snapshot for a specific pose
  void CalibrateNeutral(const Vision::FaceMeshResult &face) {
    if (!face.IsValid())
      return;

    // Brows Y (Relative to nose bridge)
    float nose_y = face.NoseBridge().y;
    float l_brow_y = face.LeftBrowInner().y;
    float r_brow_y = face.RightBrowInner().y;
    brow_y_neutral = (nose_y - l_brow_y + nose_y - r_brow_y) / 2.0f;

    // Mouth Width
    using Vision::LandmarkUtils::Distance2D;
    mouth_width_neutral =
        Distance2D(face.MouthLeftCorner(), face.MouthRightCorner());

    // Eyes (Reset Open Baseline)
    // Recalculate EAR locally or assume FaceMeshResult doesn't carry it
    // pre-calculated. For simplicity, we assume we refine "Open" here if we had
    // EAR access. We will accept "Current EAR" as passed value if implementing
    // deeply, but for now let's stick to geometric distances available in
    // FaceMeshResult.

    is_initialized = true;
  }

  void CalibrateSmile(const Vision::FaceMeshResult &face) {
    if (!face.IsValid())
      return;

    // Smile is mainly about Mouth Corners lifting
    // Calculate elevation relative to mouth center
    float center_y =
        (face.MouthUpperLipTop().y + face.MouthLowerLipBottom().y) * 0.5f;
    float l_elev = center_y - face.MouthLeftCorner().y;
    float r_elev = center_y - face.MouthRightCorner().y;

    // Store the max elevation found
    smile_elevation_max = std::max(l_elev, r_elev);

    // Ensure sensible minimum
    if (smile_elevation_max < 0.005f)
      smile_elevation_max = 0.005f;

    // Also optional: capture width if we want to use it later
    using Vision::LandmarkUtils::Distance2D;
    mouth_width_smile_max =
        Distance2D(face.MouthLeftCorner(), face.MouthRightCorner());
  }

  // Capture Eyes Closed directly (helper)
  void CalibrateEyesClosed(float avg_ear) {
    // Safety: If EAR is too high, user might not be closing eyes
    if (avg_ear > 0.25f)
      return;

    ear_closed = avg_ear;
    // Ensure it doesn't overlap with open
    if (ear_closed > ear_open - 0.05f)
      ear_closed = ear_open - 0.05f;
    if (ear_closed < 0.0f)
      ear_closed = 0.0f;
  }

  // Apply averaged calibration data
  void CalibrateFromAverage(CalibrationCommand cmd, float avg_ear,
                            float avg_mouth_w, float avg_smile_elev,
                            float avg_brow_y) {
    if (cmd == CalibrationCommand::NEUTRAL) {
      ear_open = avg_ear;
      mouth_width_neutral = avg_mouth_w;
      brow_y_neutral = avg_brow_y;
      is_initialized = true;
    } else if (cmd == CalibrationCommand::SMILE) {
      mouth_width_smile_max = avg_mouth_w;
      smile_elevation_max = avg_smile_elev;
      if (smile_elevation_max < 0.005f)
        smile_elevation_max = 0.005f;
    } else if (cmd == CalibrationCommand::BLINK_CLOSED) {
      CalibrateEyesClosed(avg_ear);
    }
  }

  // --- AUTO TUNING (The "Top du Top" feature) ---
  // Updates ranges dynamically if user exceeds them
  void UpdateAuto(float current_ear, float current_mouth_width,
                  float current_brow_elev) {
    if (!is_initialized)
      return;

    // Slow expansion for Max values (if user smiles wider than ever, expand
    // range) Rate: 0.001 (very slow adaptation)
    const float rate = 0.001f; // Slowed down from 0.01f for stability

    // Eye Wide
    if (current_ear > ear_wide) {
      ear_wide = std::lerp(ear_wide, current_ear, rate);
    }

    // Smile (if we pass current elevation or width)
    // We didn't pipe elevation to UpdateAuto yet in BlendshapeCalculator
  }
};

} // namespace Biomech
