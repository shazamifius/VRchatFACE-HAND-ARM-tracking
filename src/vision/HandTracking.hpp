#pragma once

#include <iostream>
#include <memory>
#include <onnxruntime_cxx_api.h>
#include <string>
#include <vector>

#include "../core/MathUtils.hpp"
#include <opencv2/dnn.hpp>
#include <opencv2/opencv.hpp>

namespace Vision {

struct HandResult {
  static const int NUM_LANDMARKS = 21;
  struct Landmark {
    float x, y, z;
    float visibility;
  };
  Landmark landmarks[NUM_LANDMARKS];
  float detection_confidence = 0.0f;
  bool is_right_hand = false; // To be determined by chirality or loop

  bool IsValid() const { return detection_confidence > 0.5f; }

  // Helpers for fingers
  Core::Vector3 Wrist() const {
    return {landmarks[0].x, landmarks[0].y, landmarks[0].z};
  }
  Core::Vector3 ThumbTip() const {
    return {landmarks[4].x, landmarks[4].y, landmarks[4].z};
  }
  Core::Vector3 IndexTip() const {
    return {landmarks[8].x, landmarks[8].y, landmarks[8].z};
  }
  // ...
};

class HandTracking {
public:
  HandTracking() {
    // Initialize ONNX Runtime environment
    env_ =
        std::make_unique<Ort::Env>(ORT_LOGGING_LEVEL_WARNING, "HandTracking");
    session_options_ = std::make_unique<Ort::SessionOptions>();
    session_options_->SetGraphOptimizationLevel(
        GraphOptimizationLevel::ORT_ENABLE_ALL);
  }

  void LoadModel(const std::wstring &model_path) {
    try {
      session_ = std::make_unique<Ort::Session>(*env_, model_path.c_str(),
                                                *session_options_);
      std::wcout << L"[HandTracking] Model loaded: " << model_path << L"\n";
      model_loaded_ = true;

      // Introspect
      Ort::AllocatorWithDefaultOptions allocator;
      auto input_name_ptr = session_->GetInputNameAllocated(0, allocator);
      input_name_ = std::string(input_name_ptr.get());

      auto type_info = session_->GetInputTypeInfo(0);
      auto tensor_info = type_info.GetTensorTypeAndShapeInfo();
      input_dims_ = tensor_info.GetShape();

      if (input_dims_.size() == 4) {
        input_w_ = static_cast<int>(input_dims_[3]);
        input_h_ = static_cast<int>(input_dims_[2]);
        if (input_w_ < 0)
          input_w_ = 224; // Default for hands
        if (input_h_ < 0)
          input_h_ = 224;
      }

      auto output_name_ptr = session_->GetOutputNameAllocated(0, allocator);
      output_name_ = std::string(output_name_ptr.get());
      std::cout << "[HandTracking] Input: " << input_name_ << " (" << input_w_
                << "x" << input_h_ << ")" << std::endl;

    } catch (const Ort::Exception &e) {
      std::cerr << "[HandTracking] Model load failed: " << e.what() << "\n";
      model_loaded_ = false;
    }
  }

  bool RunInference(const cv::Mat &frame, HandResult &out_hand) {
    if (!model_loaded_ || frame.empty())
      return false;

    // Resize
    cv::Mat resized;
    cv::resize(frame, resized, cv::Size(input_w_, input_h_));
    cv::cvtColor(resized, resized, cv::COLOR_BGR2RGB);

    // Blob
    cv::Mat blob = cv::dnn::blobFromImage(resized, 1.0 / 255.0, cv::Size(),
                                          cv::Scalar(0, 0, 0), false, false);

    size_t input_size = 1 * 3 * input_w_ * input_h_;
    auto mem_info =
        Ort::MemoryInfo::CreateCpu(OrtArenaAllocator, OrtMemTypeDefault);
    const char *in_names[] = {input_name_.c_str()};
    const char *out_names[] = {output_name_.c_str()};

    Ort::Value input_tensor =
        Ort::Value::CreateTensor<float>(mem_info, blob.ptr<float>(), input_size,
                                        input_dims_.data(), input_dims_.size());

    try {
      auto outputs = session_->Run(Ort::RunOptions{nullptr}, in_names,
                                   &input_tensor, 1, out_names, 1);

      float *data = outputs.front().GetTensorMutableData<float>();
      auto shape = outputs.front().GetTensorTypeAndShapeInfo().GetShape();

      return PostProcess(data, shape, out_hand);
    } catch (...) {
      return false;
    }
  }

private:
  bool PostProcess(float *data, const std::vector<int64_t> &shape,
                   HandResult &result) {
    // Hands usually output 21 landmarks * 3 = 63 floats per hand.
    // Or 1x21x3.
    int count = 1;
    for (auto s : shape)
      count *= (int)s;

    if (count < 63)
      return false;

    result.detection_confidence = 0.9f;

    for (int i = 0; i < 21; ++i) {
      float x = data[i * 3 + 0];
      float y = data[i * 3 + 1];
      float z = data[i * 3 + 2];

      // Normalize if needed
      if (x > 1.0f || y > 1.0f) {
        x /= (float)input_w_;
        y /= (float)input_h_;
        z /= (float)input_w_;
      }

      result.landmarks[i].x = x;
      result.landmarks[i].y = y;
      result.landmarks[i].z = z;
      result.landmarks[i].visibility = 1.0f;
    }
    return true;
  }

  std::unique_ptr<Ort::Env> env_;
  std::unique_ptr<Ort::SessionOptions> session_options_;
  std::unique_ptr<Ort::Session> session_;
  bool model_loaded_ = false;
  std::string input_name_ = "input_1";
  std::string output_name_ = "ld_21_3d";
  std::vector<int64_t> input_dims_ = {1, 3, 224, 224};
  int input_w_ = 224;
  int input_h_ = 224;
};

} // namespace Vision
