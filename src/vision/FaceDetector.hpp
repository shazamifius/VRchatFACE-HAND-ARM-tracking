#pragma once

#include <iostream>
#include <opencv2/opencv.hpp>
#include <vector>

namespace Vision {

class FaceDetector {
public:
  FaceDetector() {}

  bool LoadModel(const std::string &model_path) {
    try {
      // Input size will be set dynamically, but we init with VGA
      detector_ = cv::FaceDetectorYN::create(model_path, "", cv::Size(320, 320),
                                             0.6f,  // Score Threshold
                                             0.3f,  // NMS Threshold
                                             5000); // Top K
      std::cout << "[FaceDetector] Loaded YuNet model: " << model_path
                << std::endl;
      return true;
    } catch (const cv::Exception &e) {
      std::cerr << "[FaceDetector] Error loading model: " << e.what()
                << std::endl;
      return false;
    }
  }

  // Returns true if a face is found. out_rect contains the bounding box.
  bool Detect(const cv::Mat &frame, cv::Rect &out_rect) {
    if (frame.empty() || detector_.empty())
      return false;

    // Set input size if changed
    if (frame.size() != input_size_) {
      detector_->setInputSize(frame.size());
      input_size_ = frame.size();
    }

    cv::Mat faces;
    detector_->detect(frame, faces);

    if (faces.rows < 1) {
      return false;
    }

    // YuNet returns 1 row per face. Columns: x, y, w, h, ... landmarks ...
    // conf
    // We take the one with highest confidence (usually first, or sort)
    // For now, take the first one (largest/conf)

    float *data = (float *)faces.row(0).data;
    float x = data[0];
    float y = data[1];
    float w = data[2];
    float h = data[3];
    float conf = data[14];

    if (conf < 0.6f)
      return false;

    // Convert to Rect with bounds checking
    int ix = (int)x;
    int iy = (int)y;
    int iw = (int)w;
    int ih = (int)h;

    // Expand logic: MediaPipe FaceMesh usually needs a bit of margin around the
    // face
    float margin = 0.2f;
    int dx = (int)(iw * margin);
    int dy = (int)(ih * margin);

    ix -= dx;
    iy -= dy;
    iw += dx * 2;
    ih += dy * 2;

    // Clamp
    if (ix < 0)
      ix = 0;
    if (iy < 0)
      iy = 0;
    if (ix + iw > frame.cols)
      iw = frame.cols - ix;
    if (iy + ih > frame.rows)
      ih = frame.rows - iy;

    out_rect = cv::Rect(ix, iy, iw, ih);
    return true;
  }

private:
  cv::Ptr<cv::FaceDetectorYN> detector_;
  cv::Size input_size_ = cv::Size(0, 0);
};

} // namespace Vision
