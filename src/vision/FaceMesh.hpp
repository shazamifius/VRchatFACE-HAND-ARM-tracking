#pragma once

#include <iostream>
#include <memory>
#include <onnxruntime_cxx_api.h>
#include <string>
#include <vector>

#include <opencv2/dnn.hpp>
#include <opencv2/opencv.hpp>

#include "FaceTypes.hpp"

namespace Vision {

class FaceMesh {
public:
  FaceMesh() {
    // Initialize ONNX Runtime environment
    env_ = std::make_unique<Ort::Env>(ORT_LOGGING_LEVEL_WARNING, "FaceMesh");
    session_options_ = std::make_unique<Ort::SessionOptions>();

    // Optimization: Graph optimization level
    session_options_->SetGraphOptimizationLevel(
        GraphOptimizationLevel::ORT_ENABLE_ALL);
  }

  // Load Model
  void LoadModel(const std::wstring &model_path) {
    try {
      session_ = std::make_unique<Ort::Session>(*env_, model_path.c_str(),
                                                *session_options_);
      std::wcout << L"[FaceMesh] Model loaded: " << model_path << L"\n";
      model_loaded_ = true;
    } catch (const Ort::Exception &e) {
      std::cerr << "[FaceMesh] ORT Exception: " << e.what() << "\n";
      std::cerr << "[FaceMesh] Running in STUB mode (no model)\n";
      model_loaded_ = false;
    }
  }

  // Run inference and get face landmarks
  bool RunInference(const cv::Mat &frame, FaceMeshResult &out_face) {
    if (!model_loaded_) {
      // STUB MODE: Return mock neutral face for testing
      return GenerateStubFace(out_face);
    }

    if (frame.empty()) {
      return false;
    }

    // 1. Preprocessing (Resize + CHW + Normalize)
    const int input_width = 256; // Typical for MediaPipe Face Mesh
    const int input_height = 256;

    cv::Mat resized;
    cv::resize(frame, resized, cv::Size(input_width, input_height));

    // Convert to float and normalize [0, 1]
    cv::Mat dnn_blob =
        cv::dnn::blobFromImage(resized, 1.0 / 255.0, cv::Size(256, 256),
                               cv::Scalar(0, 0, 0), true, false);

    // 2. Wrap in Tensor
    std::vector<int64_t> input_shape = {1, 3, 256, 256};
    size_t input_tensor_size = 1 * 3 * 256 * 256;

    auto memory_info =
        Ort::MemoryInfo::CreateCpu(OrtArenaAllocator, OrtMemTypeDefault);

    // Input/Output Names (typical for MediaPipe models)
    const char *input_names[] = {"input"};
    const char *output_names[] = {"output"};

    Ort::Value input_tensor = Ort::Value::CreateTensor<float>(
        memory_info, dnn_blob.ptr<float>(), input_tensor_size,
        input_shape.data(), input_shape.size());

    // 3. Run
    try {
      auto output_tensors = session_->Run(Ort::RunOptions{nullptr}, input_names,
                                          &input_tensor, 1, output_names, 1);

      // 4. Post-process Face Mesh output
      float *output_data = output_tensors.front().GetTensorMutableData<float>();
      auto output_shape =
          output_tensors.front().GetTensorTypeAndShapeInfo().GetShape();

      return PostProcessFaceMesh(output_data, output_shape, out_face,
                                 frame.cols, frame.rows);

    } catch (const Ort::Exception &e) {
      // std::cerr << "[FaceMesh] Inference Error: " << e.what() << "\n";
      return false;
    }
  }

private:
  // STUB MODE: Generate neutral face for testing architecture
  bool GenerateStubFace(FaceMeshResult &result) {
    result.detection_confidence = 0.95f; // High confidence

    // Generate 468 neutral landmarks in a rough face shape
    // This is just for testing the pipeline without a real model
    for (int i = 0; i < FaceMeshResult::NUM_LANDMARKS; i++) {
      result.landmarks[i].x = 0.5f; // Center X
      result.landmarks[i].y = 0.5f; // Center Y
      result.landmarks[i].z = 0.0f; // No depth
      result.landmarks[i].visibility = 1.0f;
    }

    // Add some variation to key landmarks for testing
    // Eyes
    result.landmarks[33].x = 0.35f;
    result.landmarks[33].y = 0.4f; // Left eye
    result.landmarks[263].x = 0.65f;
    result.landmarks[263].y = 0.4f; // Right eye

    // Mouth
    result.landmarks[61].x = 0.35f;
    result.landmarks[61].y = 0.65f; // Left corner
    result.landmarks[291].x = 0.65f;
    result.landmarks[291].y = 0.65f; // Right corner
    result.landmarks[13].y = 0.63f;  // Upper lip
    result.landmarks[14].y = 0.67f;  // Lower lip

    // Nose
    result.landmarks[1].y = 0.5f; // Nose tip

    return true;
  }

  // Post-process MediaPipe Face Mesh output
  // Expected output shape: [1, 468, 3] where each landmark is [x, y, z]
  bool PostProcessFaceMesh(float *data, const std::vector<int64_t> &shape,
                           FaceMeshResult &result, int orig_width,
                           int orig_height) {
    if (shape.size() < 3) {
      std::cerr << "[FaceMesh] Unexpected output rank: " << shape.size()
                << "\n";
      return false;
    }

    int num_landmarks = static_cast<int>(shape[1]); // Should be 468
    int coords_per_landmark =
        static_cast<int>(shape[2]); // Should be 3 (x, y, z)

    if (num_landmarks != 468) {
      std::cerr << "[FaceMesh] Expected 468 landmarks, got " << num_landmarks
                << "\n";
      return false;
    }

    if (coords_per_landmark != 3) {
      std::cerr << "[FaceMesh] Expected 3 coords per landmark, got "
                << coords_per_landmark << "\n";
      return false;
    }

    result.detection_confidence = 0.95f; // TODO: Get from model if available

    // Extract landmarks
    for (int i = 0; i < num_landmarks; i++) {
      int idx = i * 3;
      result.landmarks[i].x = data[idx + 0]; // X (normalized [0,1])
      result.landmarks[i].y = data[idx + 1]; // Y (normalized [0,1])
      result.landmarks[i].z = data[idx + 2]; // Z (depth)
      result.landmarks[i].visibility = 1.0f; // Assume all visible
    }

    return true;
  }

  std::unique_ptr<Ort::Env> env_;
  std::unique_ptr<Ort::SessionOptions> session_options_;
  std::unique_ptr<Ort::Session> session_;
  bool model_loaded_ = false;
};

} // namespace Vision
