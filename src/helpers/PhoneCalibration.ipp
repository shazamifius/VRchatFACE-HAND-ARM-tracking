
// Helper to apply zero offsets (Calibration)
Biomech::ARKitBlendshapes
ApplyPhoneCalibration(const Biomech::ARKitBlendshapes &input,
                      const Biomech::ARKitBlendshapes &offsets) {
  Biomech::ARKitBlendshapes result;
  // We cannot iterate easily, so we must map them one by one or use memcopy
  // tricks if layout is guaranteed. Given the struct is just floats, we can
  // cast to float* safely IF packing is standard. However, to be safe and
  // explicit (and avoid UB), we should do it manually or via a macro. For 52
  // shapes, a macro is best.

#define CALIB_APPLY(name)                                                      \
  result.name = std::max(0.0f, input.name - offsets.name)

  // Eyes
  CALIB_APPLY(eyeBlinkLeft);
  CALIB_APPLY(eyeBlinkRight);
  CALIB_APPLY(eyeLookUpLeft);
  CALIB_APPLY(eyeLookUpRight);
  CALIB_APPLY(eyeLookDownLeft);
  CALIB_APPLY(eyeLookDownRight);
  CALIB_APPLY(eyeLookInLeft);
  CALIB_APPLY(eyeLookInRight);
  CALIB_APPLY(eyeLookOutLeft);
  CALIB_APPLY(eyeLookOutRight);
  CALIB_APPLY(eyeSquintLeft);
  CALIB_APPLY(eyeSquintRight);
  CALIB_APPLY(eyeWideLeft);
  CALIB_APPLY(eyeWideRight);

  // Jaw
  CALIB_APPLY(jawOpen);
  CALIB_APPLY(jawForward);
  CALIB_APPLY(jawLeft);
  CALIB_APPLY(jawRight);

  // Mouth
  CALIB_APPLY(mouthClose);
  CALIB_APPLY(mouthFunnel);
  CALIB_APPLY(mouthPucker);
  CALIB_APPLY(mouthLeft);
  CALIB_APPLY(mouthRight);
  CALIB_APPLY(mouthSmileLeft);
  CALIB_APPLY(mouthSmileRight);
  CALIB_APPLY(mouthFrownLeft);
  CALIB_APPLY(mouthFrownRight);
  CALIB_APPLY(mouthDimpleLeft);
  CALIB_APPLY(mouthDimpleRight);
  CALIB_APPLY(mouthStretchLeft);
  CALIB_APPLY(mouthStretchRight);
  CALIB_APPLY(mouthRollLower);
  CALIB_APPLY(mouthRollUpper);
  CALIB_APPLY(mouthShrugLower);
  CALIB_APPLY(mouthShrugUpper);
  CALIB_APPLY(mouthPressLeft);
  CALIB_APPLY(mouthPressRight);
  CALIB_APPLY(mouthLowerDownLeft);
  CALIB_APPLY(mouthLowerDownRight);
  CALIB_APPLY(mouthUpperUpLeft);
  CALIB_APPLY(mouthUpperUpRight);

  // Brows
  CALIB_APPLY(browDownLeft);
  CALIB_APPLY(browDownRight);
  CALIB_APPLY(browInnerUp);
  CALIB_APPLY(browOuterUpLeft);
  CALIB_APPLY(browOuterUpRight);

  // Cheeks
  CALIB_APPLY(cheekPuff);
  CALIB_APPLY(cheekSquintLeft);
  CALIB_APPLY(cheekSquintRight);

  // Nose
  CALIB_APPLY(noseSneerLeft);
  CALIB_APPLY(noseSneerRight);

  // Tongue
  CALIB_APPLY(tongueOut);

#undef CALIB_APPLY
  return result;
}

// Save Calibration
void SavePhoneCalibration(const Biomech::ARKitBlendshapes &offsets,
                          const std::string &path) {
  try {
    nlohmann::json j = offsets;
    std::ofstream file(path);
    file << j.dump(4);
    std::cout << "[Calibration] Saved Phone Calibration to " << path
              << std::endl;
  } catch (const std::exception &e) {
    std::cerr << "[Calibration] Error saving phone calibration: " << e.what()
              << std::endl;
  }
}

// Load Calibration
bool LoadPhoneCalibration(Biomech::ARKitBlendshapes &offsets,
                          const std::string &path) {
  try {
    std::ifstream file(path);
    if (!file.is_open())
      return false;

    nlohmann::json j;
    file >> j;
    offsets = j.get<Biomech::ARKitBlendshapes>();
    std::cout << "[Calibration] Loaded Phone Calibration from " << path
              << std::endl;
    return true;
  } catch (const std::exception &e) {
    std::cerr << "[Calibration] Error loading phone calibration: " << e.what()
              << std::endl;
    return false;
  }
}
