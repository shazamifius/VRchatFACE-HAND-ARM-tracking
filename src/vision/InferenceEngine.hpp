#pragma once

#include <iostream>
#include <memory>
#include <onnxruntime_cxx_api.h>
#include <string>
#include <vector>

#include <opencv2/dnn.hpp>
#include <opencv2/opencv.hpp>

#include "PoseTypes.hpp"

namespace Vision {

class InferenceEngine {
public:
  InferenceEngine() {
    // Initialize ONNX Runtime environment
    env_ = std::make_unique<Ort::Env>(ORT_LOGGING_LEVEL_WARNING,
                                      "VRChatUniversalBridge");
    session_options_ = std::make_unique<Ort::SessionOptions>();

    // Optimization: Graph optimization level
    session_options_->SetGraphOptimizationLevel(
        GraphOptimizationLevel::ORT_ENABLE_ALL);

    // TODO: Enable DirectML if available
    // For now we default to CPU, but we'll add logic to try DirectML
  }

  // Load Model
  void LoadModel(const std::wstring &model_path) {
    try {
      session_ = std::make_unique<Ort::Session>(*env_, model_path.c_str(),
                                                *session_options_);
      std::wcout << L"[Vision] Model loaded: " << model_path << L"\n";

      // Get Input/Output metadata (Simplified for now)
      // Allocating names buffers if needed...
    } catch (const Ort::Exception &e) {
      std::cerr << "[Vision] ORT Exception: " << e.what() << "\n";
    }
  }

  // Pre-process and Run - Returns detected pose
  bool RunInference(const cv::Mat &frame, PoseResult &out_pose) {
    if (!session_ || frame.empty())
      return false;

    // 1. Preprocessing (Resize + CHW + Normalize)
    // Assume defaults: 640x640, float32, [0,1] or standard means
    // TODO: Read input shape from model metadata
    const int input_width = 640;
    const int input_height = 640;

    cv::Mat resized;
    cv::resize(frame, resized, cv::Size(input_width, input_height));

    // Convert to float and normalize [0, 1]
    resized.convertTo(resized, CV_32F, 1.0f / 255.0f);

    // HWC to CHW
    // std::vector<float> input_tensor_values(1 * 3 * 640 * 640);
    // OpenCV gives BGR/RGB interleaved.
    // We need Planar.

    // Optimized blobFromImage (OpenCV DNN module does this too, but we do
    // manual for control)
    cv::Mat dnn_blob =
        cv::dnn::blobFromImage(frame, 1.0 / 255.0, cv::Size(640, 640),
                               cv::Scalar(0, 0, 0), true, false);

    // 2. Wrap in Tensor
    std::vector<int64_t> input_shape = {1, 3, 640, 640};
    size_t input_tensor_size = 1 * 3 * 640 * 640;

    auto memory_info =
        Ort::MemoryInfo::CreateCpu(OrtArenaAllocator, OrtMemTypeDefault);

    // Input/Output Names (Hardcoded for YOLOv8 Pose usually "images" ->
    // "output0") Use lookup for robustness in future
    const char *input_names[] = {"images"};
    const char *output_names[] = {"output0"};

    Ort::Value input_tensor = Ort::Value::CreateTensor<float>(
        memory_info, dnn_blob.ptr<float>(), input_tensor_size,
        input_shape.data(), input_shape.size());

    // 3. Run
    try {
      auto output_tensors = session_->Run(Ort::RunOptions{nullptr}, input_names,
                                          &input_tensor, 1, output_names, 1);

      // 4. Post-process YOLOv8-Pose output
      float *output_data = output_tensors.front().GetTensorMutableData<float>();
      auto output_shape =
          output_tensors.front().GetTensorTypeAndShapeInfo().GetShape();

      return PostProcessPose(output_data, output_shape, out_pose, frame.cols,
                             frame.rows);

    } catch (const Ort::Exception &e) {
      // std::cerr << "Inference Error: " << e.what() << "\n";
      return false;
    }
  }

private:
  // Post-process YOLOv8-Pose output
  // Output shape: [1, 56, 8400]
  // 56 = 4 (bbox) + 1 (conf) + 51 (17 keypoints * 3 [x, y, conf])
  bool PostProcessPose(float *data, const std::vector<int64_t> &shape,
                       PoseResult &result, int orig_width, int orig_height) {
    if (shape.size() < 3)
      return false;

    int num_channels = static_cast<int>(shape[1]);   // Should be 56
    int num_detections = static_cast<int>(shape[2]); // Should be 8400

    if (num_channels != 56) {
      std::cerr << "[Vision] Unexpected output shape: " << num_channels
                << " channels\n";
      return false;
    }

    // Find detection with highest confidence
    float max_conf = 0.0f;
    int best_idx = -1;

    for (int i = 0; i < num_detections; i++) {
      float obj_conf =
          data[4 * num_detections + i]; // Index 4 = object confidence
      if (obj_conf > max_conf) {
        max_conf = obj_conf;
        best_idx = i;
      }
    }

    if (best_idx == -1 || max_conf < 0.3f) {
      return false; // No valid detection
    }

    result.detection_confidence = max_conf;

    // Extract bounding box (for scaling keypoints)
    float bbox_x = data[0 * num_detections + best_idx];
    float bbox_y = data[1 * num_detections + best_idx];
    float bbox_w = data[2 * num_detections + best_idx];
    float bbox_h = data[3 * num_detections + best_idx];

    // Extract 17 keypoints (start at index 5)
    // Each keypoint has 3 values: x, y, confidence
    for (int k = 0; k < 17; k++) {
      int x_idx = (5 + k * 3 + 0) * num_detections + best_idx;
      int y_idx = (5 + k * 3 + 1) * num_detections + best_idx;
      int conf_idx = (5 + k * 3 + 2) * num_detections + best_idx;

      // Coordinates are in 640x640 space, scale back to original
      result.keypoints[k].x = data[x_idx] * orig_width / 640.0f;
      result.keypoints[k].y = data[y_idx] * orig_height / 640.0f;
      result.keypoints[k].confidence = data[conf_idx];
    }

    return true;
  }

private:
  std::unique_ptr<Ort::Env> env_;
  std::unique_ptr<Ort::SessionOptions> session_options_;
  std::unique_ptr<Ort::Session> session_;
};

} // namespace Vision
