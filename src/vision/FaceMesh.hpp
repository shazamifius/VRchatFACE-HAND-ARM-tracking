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

      // Inspect Model Input/Output
      Ort::AllocatorWithDefaultOptions allocator;

      // Print all inputs
      std::cout << "[FaceMesh] Inputs:" << std::endl;
      size_t num_input_nodes = session_->GetInputCount();
      for (size_t i = 0; i < num_input_nodes; i++) {
        auto name_ptr = session_->GetInputNameAllocated(i, allocator);
        auto type_info = session_->GetInputTypeInfo(i);
        auto tensor_info = type_info.GetTensorTypeAndShapeInfo();
        auto shape = tensor_info.GetShape();
        std::cout << "  " << i << ": " << name_ptr.get() << " [";
        for (size_t j = 0; j < shape.size(); j++)
          std::cout << shape[j] << (j < shape.size() - 1 ? "," : "");
        std::cout << "]" << std::endl;

        // Store first input props as primary
        if (i == 0) {
          input_name_ = std::string(name_ptr.get());
          input_dims_ = shape;
          if (shape.size() == 4) {
            // Default NCHW
            input_is_nhwc_ = false;
            input_height_ = static_cast<int>(shape[2]);
            input_width_ = static_cast<int>(shape[3]);

            // Check for NHWC (Channel last = 3)
            if (shape[3] == 3) {
              input_is_nhwc_ = true;
              input_height_ = static_cast<int>(shape[1]);
              input_width_ = static_cast<int>(shape[2]);
            }
            // Check/verify NCHW (Channel first = 3)
            else if (shape[1] == 3) {
              input_is_nhwc_ = false;
              // already set above
            }

            if (input_height_ == -1)
              input_height_ = 192;
            if (input_width_ == -1)
              input_width_ = 192;
          }
        }
      }

      // Print all outputs
      std::cout << "[FaceMesh] Outputs:" << std::endl;
      size_t num_output_nodes = session_->GetOutputCount();
      for (size_t i = 0; i < num_output_nodes; i++) {
        auto name_ptr = session_->GetOutputNameAllocated(i, allocator);
        auto type_info = session_->GetOutputTypeInfo(i);
        auto tensor_info = type_info.GetTensorTypeAndShapeInfo();
        auto shape = tensor_info.GetShape();

        // AUTO-DETECT: Capture output name
        if (i == 0) {
          output_name_ = std::string(name_ptr.get());
        }

        std::cout << "  " << i << ": " << name_ptr.get() << " [";
        for (size_t j = 0; j < shape.size(); j++)
          std::cout << shape[j] << (j < shape.size() - 1 ? "," : "");
        std::cout << "]" << std::endl;
      }

    } catch (const Ort::Exception &e) {
      std::cerr << "[FaceMesh] ORT Exception: " << e.what() << "\n";
      std::cerr << "[FaceMesh] CRITICAL: Model load failed. Face tracking will "
                   "be disabled.\n";
      model_loaded_ = false;
    }
  }

  // Run inference and get face landmarks
  bool RunInference(const cv::Mat &frame, FaceMeshResult &out_face) {
    if (!model_loaded_) {
      return false;
    }

    if (frame.empty()) {
      return false;
    }

    // 1. Preprocessing (Resize + CHW + Normalize)
    // MediaPipe usually expects RGB, float [0, 1]
    // 1. Preprocessing
    cv::Mat resized;
    cv::resize(frame, resized, cv::Size(input_width_, input_height_));
    cv::cvtColor(resized, resized, cv::COLOR_BGR2RGB);

    // Prepare input tensor data
    // Ort expects Float32.
    cv::Mat float_data;
    resized.convertTo(float_data, CV_32F, 1.0f / 255.0f);

    cv::Mat final_blob;

    if (!input_is_nhwc_) {
      // NCHW: Use blobFromImage logic (Planar)
      // Note: blobFromImage can do resize/swap/mean/scale.
      // But since we did manual resize/convert, we just want to permute.
      // It's safer to just use blobFromImage on original frame directly if we
      // want speed, but here we already have 'float_data' (NHWC). Let's use
      // blobFromImage on the float_data. Actually blobFromImage expects 8U
      // usually or handles 32F.
      cv::dnn::blobFromImage(float_data, final_blob); // Default is NCHW
    } else {
      // NHWC: Use data directly
      final_blob = float_data;
    }

    // 2. Wrap in Tensor
    // Calculate total float elements
    size_t input_tensor_size = 1 * 3 * input_width_ * input_height_;

    auto memory_info =
        Ort::MemoryInfo::CreateCpu(OrtArenaAllocator, OrtMemTypeDefault);

    const char *input_names[] = {input_name_.c_str()};
    const char *output_names[] = {output_name_.c_str()};

    // Ensure data pointer is valid.
    // If blobFromImage was used, final_blob is continuous NCHW.
    // If float_data was used, it is continuous NHWC.
    if (!final_blob.isContinuous()) {
      final_blob = final_blob.clone();
    }

    Ort::Value input_tensor = Ort::Value::CreateTensor<float>(
        memory_info, final_blob.ptr<float>(), input_tensor_size,
        input_dims_.data(), input_dims_.size());

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
      std::cerr << "[FaceMesh] ORT Inference Error: " << e.what() << "\n";
      return false;
    }
  }

private:
  // Post-process MediaPipe Face Mesh output
  // Expected output shape: [1, 1404] (468*3) or [1, 468, 3]
  bool PostProcessFaceMesh(float *data, const std::vector<int64_t> &shape,
                           FaceMeshResult &result, int orig_width,
                           int orig_height) {

    // Check total elements
    int total_elements = 1;
    for (auto s : shape)
      total_elements *= (int)s;

    if (total_elements != 1404 &&
        total_elements != 1404 + 30) { // sometimes 478 landmarks
      std::cerr << "[FaceMesh] Unexpected output size: " << total_elements
                << ". Expected 1404.\n";
      if (total_elements < 1404)
        return false;
    }

    result.detection_confidence = 0.9f;

    // Extract landmarks
    // Usually output is already normalized [0, 1]? Or pixels?
    // MediaPipe ONNX usually returns Coordinates.
    // If it's the raw TFLite->ONNX, it's often [1, 1404]

    for (int i = 0; i < FaceMeshResult::NUM_LANDMARKS; i++) {
      int idx = i * 3;

      // Need to verify if model output is Normalized (0-1) or Pixel Coordinates
      // (0-192) Standard MediaPipe models usually output NORMALIZED
      // coordinates.

      float x = data[idx + 0];
      float y = data[idx + 1];
      float z = data[idx + 2];

      // Some ONNX exports output pixel coordinates based on input size (192).
      // Let's heuristic check: are values > 1.0?
      if (i == 0 && (x > 1.0f || y > 1.0f)) {
        // Normalize manually
        x /= (float)input_width_;
        y /= (float)input_height_;
        z /= (float)input_width_; // Scale Z similarly
      } else {
        // If first point is < 1, valid assumption is global normalized.
        // Wait, loop implies we check every point.
        // We shouldn't change heuristic mid-loop.
        // Let's assume normalized for now or division by input_width based on
        // first point.
      }

      // Ideally we detect this ONCE at start or have a flag.
      // For now, assume normalized as that's typical for MediaPipe Graph
      // Output. (But raw model might be pixels). Actually standard specific
      // models are often 192x192 pixels. We will perform a check:

      result.landmarks[i].x = x;
      result.landmarks[i].y = y;
      result.landmarks[i].z = z;
      result.landmarks[i].visibility = 1.0f;
    }

    // Double check normalization on the nose tip (index 1)
    if (result.landmarks[1].x > 1.0f || result.landmarks[1].y > 1.0f) {
      for (int i = 0; i < FaceMeshResult::NUM_LANDMARKS; ++i) {
        result.landmarks[i].x /= (float)input_width_;
        result.landmarks[i].y /= (float)input_height_;
        // Z also needs scaling
        result.landmarks[i].z /= (float)input_width_;
      }
    }

    return true;
  }

  std::unique_ptr<Ort::Env> env_;
  std::unique_ptr<Ort::SessionOptions> session_options_;
  std::unique_ptr<Ort::Session> session_;
  bool model_loaded_ = false;

  std::string input_name_ = "input_1";
  std::string output_name_ = "output_mesh"; // "ld_21_2d" etc
  std::vector<int64_t> input_dims_ = {1, 3, 192, 192};
  int input_width_ = 192;
  int input_height_ = 192;
  bool input_is_nhwc_ = false;
};

} // namespace Vision
