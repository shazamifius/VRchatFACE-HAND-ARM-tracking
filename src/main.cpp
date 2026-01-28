#include <atomic>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <iostream>
#include <mutex>
#include <thread>
#include <vector>

// clang-format off
#include <glad/glad.h>
#include <GLFW/glfw3.h>
// clang-format on

#include "core/MathUtils.hpp"
#include "core/Profiler.hpp"
#include "vision/FaceMesh.hpp"
#include "vision/HandTracking.hpp"
#include "vision/InferenceEngine.hpp"
#include "vision/ModelSelector.hpp"

#include "biomech/BlendshapeTypes.hpp"
#include "biomech/CoordinateConverter.hpp"
#include "core/AutoConfig.hpp"
#include "core/CameraState.hpp"
#include "network/OSCClient.hpp"
#include "network/VMCProtocol.hpp"
#include "ui/MainWindow.hpp"

// OpenCV
#include <opencv2/opencv.hpp>

// ImGui Backends
#include "imgui.h"
#include "imgui_impl_glfw.h"
#include "imgui_impl_opengl3.h"

// --- SHARED STATE (Thread-Safe) ---
struct SharedState {
  std::mutex mutex;
  cv::Mat latest_frame;
  bool new_frame_ready = false;

  // Tracking Data
  Core::Vector3 head_pos = {0, 0, 0};
  Core::Quaternion head_rot = {1, 0, 0, 0};

  // Face Tracking Data (NEW)
  Biomech::ARKitBlendshapes blendshapes;
  bool face_tracking_active = false;

  // Debug / Test State
  UI::BodyState body_debug;

  // Stats
  long long inference_time_us = 0;
  long long face_inference_time_us = 0; // NEW: Face tracking latency
  bool system_running = true;

  // Camera Control
  int pending_camera_index = -1;
  bool camera_available[5] = {false, false, false, false,
                              false}; // Status for Camera 0-4

  // Network Info
  std::string server_ip = "127.0.0.1";
  int server_port = 8080;

  // Cloudflare Tunnel Info (updated asynchronously)
  std::string tunnel_url = "";
  bool tunnel_ready = false;
};

SharedState g_appState;
CameraState g_camera; // 3D camera controls

// ... (Keeping existing includes)
#include "biomech/SkeletonSolver.hpp"

// Helper to map enum to VMC string
std::string BoneToString(Biomech::HumanBodyBones bone) {
  using namespace Biomech;
  switch (bone) {
  case HumanBodyBones::Hips:
    return "Hips";
  case HumanBodyBones::Spine:
    return "Spine";
  case HumanBodyBones::Chest:
    return "Chest";
  case HumanBodyBones::UpperChest:
    return "UpperChest";
  case HumanBodyBones::Neck:
    return "Neck";
  case HumanBodyBones::Head:
    return "Head";

  case HumanBodyBones::LeftShoulder:
    return "LeftShoulder";
  case HumanBodyBones::LeftUpperArm:
    return "LeftUpperArm";
  case HumanBodyBones::LeftLowerArm:
    return "LeftLowerArm";
  case HumanBodyBones::LeftHand:
    return "LeftHand";

  case HumanBodyBones::RightShoulder:
    return "RightShoulder";
  case HumanBodyBones::RightUpperArm:
    return "RightUpperArm";
  case HumanBodyBones::RightLowerArm:
    return "RightLowerArm";
  case HumanBodyBones::RightHand:
    return "RightHand";

  case HumanBodyBones::LeftThumbProximal:
    return "LeftThumbProximal";
  case HumanBodyBones::LeftThumbIntermediate:
    return "LeftThumbIntermediate";
  case HumanBodyBones::LeftThumbDistal:
    return "LeftThumbDistal";
  case HumanBodyBones::LeftIndexProximal:
    return "LeftIndexProximal";
  case HumanBodyBones::LeftIndexIntermediate:
    return "LeftIndexIntermediate";
  case HumanBodyBones::LeftIndexDistal:
    return "LeftIndexDistal";
  case HumanBodyBones::LeftMiddleProximal:
    return "LeftMiddleProximal";
  case HumanBodyBones::LeftMiddleIntermediate:
    return "LeftMiddleIntermediate";
  case HumanBodyBones::LeftMiddleDistal:
    return "LeftMiddleDistal";
  case HumanBodyBones::LeftRingProximal:
    return "LeftRingProximal";
  case HumanBodyBones::LeftRingIntermediate:
    return "LeftRingIntermediate";
  case HumanBodyBones::LeftRingDistal:
    return "LeftRingDistal";
  case HumanBodyBones::LeftLittleProximal:
    return "LeftLittleProximal";
  case HumanBodyBones::LeftLittleIntermediate:
    return "LeftLittleIntermediate";
  case HumanBodyBones::LeftLittleDistal:
    return "LeftLittleDistal";

  case HumanBodyBones::RightThumbProximal:
    return "RightThumbProximal";
  case HumanBodyBones::RightThumbIntermediate:
    return "RightThumbIntermediate";
  case HumanBodyBones::RightThumbDistal:
    return "RightThumbDistal";
  case HumanBodyBones::RightIndexProximal:
    return "RightIndexProximal";
  case HumanBodyBones::RightIndexIntermediate:
    return "RightIndexIntermediate";
  case HumanBodyBones::RightIndexDistal:
    return "RightIndexDistal";
  case HumanBodyBones::RightMiddleProximal:
    return "RightMiddleProximal";
  case HumanBodyBones::RightMiddleIntermediate:
    return "RightMiddleIntermediate";
  case HumanBodyBones::RightMiddleDistal:
    return "RightMiddleDistal";
  case HumanBodyBones::RightRingProximal:
    return "RightRingProximal";
  case HumanBodyBones::RightRingIntermediate:
    return "RightRingIntermediate";
  case HumanBodyBones::RightRingDistal:
    return "RightRingDistal";
  case HumanBodyBones::RightLittleProximal:
    return "RightLittleProximal";
  case HumanBodyBones::RightLittleIntermediate:
    return "RightLittleIntermediate";
  case HumanBodyBones::RightLittleDistal:
    return "RightLittleDistal";

  default:
    return "";
  }
}

// --- VISION THREAD ---
#include "network/CloudflareTunnel.hpp"
#include "network/VideoReceiver.hpp"
#include "network/WebServer.hpp"

// Global receiver to bridge threads (could be passed via args, but shared state
// legacy)
Network::VideoReceiver g_videoReceiver;

void VisionLoop() {
  std::cout << "[Thread] Vision Thread Started." << std::endl;

  // Init components locally to thread
  Vision::InferenceEngine ai_engine;
  ai_engine.LoadModel(L"models/yolov8n-pose.onnx");

  // Face Mesh
  Vision::FaceMesh face_engine;
  face_engine.LoadModel(L"models/Facial-Landmark-Detection.onnx");
  Biomech::BlendshapeCalculator blendshape_calc;

  // Hand Tracking
  Vision::HandTracking hand_engine;
  hand_engine.LoadModel(L"models/MediaPipeHandDetector.onnx");

  Vision::ModelSelector quality_selector;
  Biomech::CoordinateConverter converter;
  Biomech::SkeletonSolver skeleton_solver;
  Network::OSCClient osc_client("127.0.0.1", 9000);

  // --- Start Web Server ---
  Network::WebServer web_server(8080);
  std::cout << "[Network] Starting Web Server on port 8080..." << std::endl;
  web_server.Start("assets/web");

  // Inject VideoReceiver to handle incoming phone camera frames
  web_server.SetVideoReceiver(&g_videoReceiver);

  std::string local_ip = web_server.GetLocalIP();
  std::cout << "[Network] Local IP: " << local_ip << std::endl;

  // Update Shared State with IP/Port for UI
  {
    std::lock_guard<std::mutex> lock(g_appState.mutex);
    g_appState.server_ip = local_ip;
    g_appState.server_port = 8080;
  }

  cv::VideoCapture cap(0);
  if (!cap.isOpened()) {
    std::cerr << "[Error] Camera not found in Vision Thread!" << std::endl;
  }

  // --- Optimization State ---
  Vision::PoseResult last_pose;
  bool last_pose_detected = false;
  int frame_count = 0;
  const int YOLO_INTERVAL = 3;

  while (true) {
    // Check exit condition
    {
      std::lock_guard<std::mutex> lock(g_appState.mutex);
      if (!g_appState.system_running)
        break;
    }

    // Check for Camera Switch logic (Local vs Network)
    // If Network has frame, use it? Or manual switch?
    // Let's prioritize Network if connected.

    cv::Mat frame;
    bool using_network = false;

    if (g_videoReceiver.GetLatestFrame(frame)) {
      using_network = true;
    } else {
      // Fallback to local camera
      {
        std::lock_guard<std::mutex> lock(g_appState.mutex);
        if (g_appState.pending_camera_index != -1) {
          std::cout << "[Thread] Switching to Camera "
                    << g_appState.pending_camera_index << std::endl;
          cap.open(g_appState.pending_camera_index);
          g_appState.pending_camera_index = -1;
        }
      }
      if (cap.isOpened()) {
        cap >> frame;
      }
    }

    if (!frame.empty()) {
      frame_count++;
      auto start_inf = std::chrono::high_resolution_clock::now();

      // Mirror if local (Network is usually mirrored by client or needs
      // specific handling)
      if (!using_network) {
        // cv::flip(frame, frame, 1); // User preference? Usually mirror for
        // self-view.
      }

      // 1. Adaptive Body Tracking (YOLO)
      if (frame_count % YOLO_INTERVAL == 0 || !last_pose_detected) {
        last_pose_detected = ai_engine.RunInference(frame, last_pose);
      }

      bool pose_detected = last_pose_detected;
      Vision::PoseResult &pose = last_pose;

      auto end_inf = std::chrono::high_resolution_clock::now();
      long long duration =
          std::chrono::duration_cast<std::chrono::microseconds>(end_inf -
                                                                start_inf)
              .count();

      long long face_duration = 0;

      // 2. Face Tracking setup
      Vision::FaceMeshResult face_result;
      bool face_detected = false;
      Biomech::ARKitBlendshapes blendshapes;

      // 3. Hand Tracking setup
      Vision::HandResult left_hand_result;
      Vision::HandResult right_hand_result;

      cv::Rect face_rect(0, 0, 0, 0);

      if (pose_detected) {
        // ... (Keep existing Logic)
        auto nose = pose.Nose();
        int face_size = (int)(frame.cols * 0.4f);
        int fx = (int)nose.x - face_size / 2;
        int fy = (int)nose.y - face_size / 2;
        // Clamp checks...
        if (fx < 0)
          fx = 0;
        if (fy < 0)
          fy = 0;
        if (fx + face_size > frame.cols)
          face_size = frame.cols - fx;
        if (fy + face_size > frame.rows)
          face_size = frame.rows - fy;
        face_rect = cv::Rect(fx, fy, face_size, face_size);

        if (face_rect.area() > 0) {
          cv::Mat face_roi = frame(face_rect);
          face_detected = face_engine.RunInference(face_roi, face_result);
          if (face_detected) {
            // Un-normalize landmarks to Global Frame
            for (auto &lm : face_result.landmarks) {
              lm.x = fx + lm.x * face_size;
              lm.y = fy + lm.y * face_size;
              lm.z = lm.z * face_size;
            }
            blendshapes = blendshape_calc.Calculate(face_result);
          }
        }

        auto ProcessHand = [&](const Vision::PoseKeypoint &wrist, bool isLeft,
                               Vision::HandResult &result) {
          if (wrist.confidence > 0.3f) {
            int size = (int)(frame.cols * 0.25f); // 25% of screen width
            int x = (int)wrist.x - size / 2;
            int y = (int)wrist.y - size / 2;

            // Clamp
            if (x < 0)
              x = 0;
            if (y < 0)
              y = 0;
            if (x + size > frame.cols)
              size = frame.cols - x;
            if (y + size > frame.rows)
              size = frame.rows - y;

            if (size > 20) {
              cv::Mat roi = frame(cv::Rect(x, y, size, size));
              if (hand_engine.RunInference(roi, result)) {
                // Un-normalize
                for (auto &lm : result.landmarks) {
                  lm.x = x + lm.x * size;
                  lm.y = y + lm.y * size;
                  lm.z = lm.z * size;
                }
                result.is_right_hand = !isLeft;
              }
            }
          }
        };

        ProcessHand(pose.LeftWrist(), true, left_hand_result);
        ProcessHand(pose.RightWrist(), false, right_hand_result);
      }

      // 4. Solve Skeleton (Body + Hands)
      auto skeleton_pose =
          skeleton_solver.Solve(pose, left_hand_result, right_hand_result);

      // 5. Send OSC
      for (const auto &bone : skeleton_pose) {
        std::string boneName = BoneToString(bone.first);
        if (!boneName.empty()) {
          Core::Vector3 unityPos =
              converter.ConvertPosition(bone.second.position);
          Core::Quaternion unityRot =
              converter.ConvertRotation(bone.second.rotation);

          if (bone.first == Biomech::HumanBodyBones::Hips) {
            unityPos.x = (unityPos.x / frame.cols - 0.5f) * 2.0f;
            unityPos.y = (1.0f - (unityPos.y / frame.rows)) * 2.0f;
            unityPos.z = 0;
          }

          osc_client.Send(
              Network::VMCProtocol::PackBonePos(boneName, unityPos, unityRot));
        }
      }

      // Face Blendshapes
      if (face_detected) {
        osc_client.SendFloat("/avatar/parameters/FT/v2/EyeClosedLeft",
                             blendshapes.eyeBlinkLeft);
        osc_client.SendFloat("/avatar/parameters/FT/v2/EyeClosedRight",
                             blendshapes.eyeBlinkRight);
        osc_client.SendFloat("/avatar/parameters/FT/v2/JawOpen",
                             blendshapes.jawOpen);
        osc_client.SendFloat("/avatar/parameters/FT/v2/MouthSmileLeft",
                             blendshapes.mouthSmileLeft);
        osc_client.SendFloat("/avatar/parameters/FT/v2/MouthSmileRight",
                             blendshapes.mouthSmileRight);
        osc_client.SendFloat("/avatar/parameters/FT/v2/BrowInnerUp",
                             blendshapes.browInnerUp);
      }

      // 6. Update Shared State
      {
        std::lock_guard<std::mutex> lock(g_appState.mutex);
        g_appState.latest_frame = frame.clone();
        g_appState.new_frame_ready = true;
        g_appState.inference_time_us = duration;
        g_appState.face_inference_time_us = face_duration;
        g_appState.blendshapes = blendshapes;
        g_appState.face_tracking_active = face_detected;

        // Pass IP to UI via State if needed. (Hack for now)
        // Ideally g_appState should have std::string server_ip.
      }
    } else {
      std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
  }

  web_server.Stop();
  std::cout << "[Thread] Vision Thread Stopped." << std::endl;
}

// --- RENDERING HELPERS ---

// Helper to upload OpenCV mat to GL Texture
void UpdateGLTexture(const cv::Mat &mat, GLuint &textureID) {
  if (mat.empty())
    return;

  if (textureID == 0) {
    glGenTextures(1, &textureID);
  }

  glBindTexture(GL_TEXTURE_2D, textureID);

  // Use BGR to RGBA conversion
  cv::Mat matRGBA;
  cv::cvtColor(mat, matRGBA, cv::COLOR_BGR2RGBA);

  glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
  glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
  glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
  glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);

  glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, matRGBA.cols, matRGBA.rows, 0,
               GL_RGBA, GL_UNSIGNED_BYTE, matRGBA.data);
}

// FBO for 3D Preview
struct OffscreenBuffer {
  GLuint fbo = 0;
  GLuint texture = 0;
  GLuint rbo = 0;
  int width = 512;
  int height = 512;

  void Init(int w, int h) {
    width = w;
    height = h;
    // Clean up if re-init
    if (fbo)
      glDeleteFramebuffers(1, &fbo);
    if (texture)
      glDeleteTextures(1, &texture);
    if (rbo)
      glDeleteRenderbuffers(1, &rbo);

    glGenFramebuffers(1, &fbo);
    glBindFramebuffer(GL_FRAMEBUFFER, fbo);

    glGenTextures(1, &texture);
    glBindTexture(GL_TEXTURE_2D, texture);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGB, width, height, 0, GL_RGB,
                 GL_UNSIGNED_BYTE, NULL);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D,
                           texture, 0);

    glGenRenderbuffers(1, &rbo);
    glBindRenderbuffer(GL_RENDERBUFFER, rbo);
    glRenderbufferStorage(GL_RENDERBUFFER, GL_DEPTH24_STENCIL8, width, height);
    glFramebufferRenderbuffer(GL_FRAMEBUFFER, GL_DEPTH_STENCIL_ATTACHMENT,
                              GL_RENDERBUFFER, rbo);

    if (glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE)
      std::cerr << "ERROR::FRAMEBUFFER:: Framebuffer is not complete!"
                << std::endl;

    glBindFramebuffer(GL_FRAMEBUFFER, 0);
  }
};

void RenderSkeleton(const OffscreenBuffer &buffer, const Core::Vector3 &pos,
                    const Core::Quaternion &rot,
                    const UI::BodyState &debug_appState) {
  if (buffer.fbo == 0)
    return;

  glBindFramebuffer(GL_FRAMEBUFFER, buffer.fbo);
  glViewport(0, 0, buffer.width, buffer.height);
  glClearColor(0.05f, 0.05f, 0.08f, 1.0f); // Sleek Dark background
  glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

  // RESET STATE: ImGui modifies this, so we must reset to Fixed Function logic
  glUseProgram(0);
  glEnable(GL_DEPTH_TEST);

  glMatrixMode(GL_PROJECTION);
  glLoadIdentity();
  float aspect = (float)buffer.width / (float)buffer.height;
  float fov = 45.0f * 3.14159f / 180.0f;
  float nearPlane = 0.1f;
  float farPlane = 100.0f;
  float top = nearPlane * tan(fov * 0.5f);
  float bottom = -top;
  float right = top * aspect;
  float left = -right;
  glFrustum(left, right, bottom, top, nearPlane, farPlane);

  glMatrixMode(GL_MODELVIEW);
  glLoadIdentity();
  // Camera position (controlled by WASDQER keys)
  glTranslatef(0.0f, g_camera.height, -g_camera.distance);
  glRotatef(g_camera.angle_y, 1.0f, 0.0f, 0.0f); // Pitch
  glRotatef(g_camera.angle_x, 0.0f, 1.0f, 0.0f); // Yaw

  // --- FLOOR GRID REMOVED based on user request ---
  /*
  glBegin(GL_LINES);
  glColor3f(0.2f, 0.2f, 0.3f);
  for (int i = -10; i <= 10; ++i) {
    glVertex3f((float)i, 0.0f, -10.0f);
    glVertex3f((float)i, 0.0f, 10.0f);
    glVertex3f(-10.0f, 0.0f, (float)i);
    glVertex3f(10.0f, 0.0f, (float)i);
  }
  glEnd();
  */

  // --- SKELETON DRAWING ---
  glPushMatrix();

  // 1. Head Transform
  // If Test Mode is active, we might override rotation here if we had sliders
  // for it. For now we use the tracked (or mock) head position/rot.
  glTranslatef(pos.x, pos.y, pos.z);

  // HEAD (Sphere/Cube)
  glColor3f(0.0f, 1.0f, 0.9f); // Cyan
  // Draw Head Box
  float s = 0.12f;
  glLineWidth(2.0f);

  // Custom "Wireframe Head"
  glBegin(GL_LINES);
  // Box
  glVertex3f(-s, -s, s);
  glVertex3f(s, -s, s);
  glVertex3f(s, -s, s);
  glVertex3f(s, s, s);
  glVertex3f(s, s, s);
  glVertex3f(-s, s, s);
  glVertex3f(-s, s, s);
  glVertex3f(-s, -s, s);

  glVertex3f(-s, -s, -s);
  glVertex3f(s, -s, -s);
  glVertex3f(s, -s, -s);
  glVertex3f(s, s, -s);
  glVertex3f(s, s, -s);
  glVertex3f(-s, s, -s);
  glVertex3f(-s, s, -s);
  glVertex3f(-s, -s, -s);

  glVertex3f(-s, -s, s);
  glVertex3f(-s, -s, -s);
  glVertex3f(s, -s, s);
  glVertex3f(s, -s, -s);
  glVertex3f(s, s, s);
  glVertex3f(s, s, -s);
  glVertex3f(-s, s, s);
  glVertex3f(-s, s, -s);
  glEnd();

  // 2. FACE FEATURES (Debug Controlled)

  // EYES
  float eye_y = 0.03f;
  float eye_x = 0.05f;
  float blink_l = debug_appState.test_mode ? debug_appState.blink_l : 0.0f;
  float blink_r = debug_appState.test_mode ? debug_appState.blink_r : 0.0f;

  // Left Eye
  glColor3f(1.0f, 0.2f, 0.6f); // Pink
  glPointSize(8.0f *
              (1.0f - blink_l)); // Shrink point to simulate blink, or draw line
  if (blink_l < 0.9f) {
    glBegin(GL_POINTS);
    glVertex3f(-eye_x, eye_y, s);
    glEnd();
  } else {
    glBegin(GL_LINES);
    glVertex3f(-eye_x - 0.02f, eye_y, s);
    glVertex3f(-eye_x + 0.02f, eye_y, s);
    glEnd();
  }

  // Right Eye
  glPointSize(8.0f * (1.0f - blink_r));
  if (blink_r < 0.9f) {
    glBegin(GL_POINTS);
    glVertex3f(eye_x, eye_y, s);
    glEnd();
  } else {
    glBegin(GL_LINES);
    glVertex3f(eye_x - 0.02f, eye_y, s);
    glVertex3f(eye_x + 0.02f, eye_y, s);
    glEnd();
  }

  // MOUTH (Jaw Open)
  float jaw = debug_appState.test_mode ? debug_appState.jaw_open : 0.0f;
  float mouth_y = -0.05f - (jaw * 0.05f); // Move down
  glColor3f(1.0f, 1.0f, 1.0f);
  glBegin(GL_LINE_LOOP);
  glVertex3f(-0.03f, -0.05f, s);  // Upper Lip L
  glVertex3f(0.03f, -0.05f, s);   // Upper Lip R
  glVertex3f(0.02f, mouth_y, s);  // Lower Lip R
  glVertex3f(-0.02f, mouth_y, s); // Lower Lip L
  glEnd();

  // TONGUE
  float tongue = debug_appState.test_mode ? debug_appState.tongue_out : 0.0f;
  if (tongue > 0.1f) {
    glColor3f(1.0f, 0.4f, 0.4f); // Reddish
    glBegin(GL_LINES);
    glVertex3f(0.0f, mouth_y + 0.01f, s);
    glVertex3f(0.0f, mouth_y - 0.01f - (tongue * 0.08f),
               s + 0.05f); // Stick out and down
    glEnd();
  }

  glPopMatrix(); // End Head

  // 3. BODY & ARMS (Test Mode Animation)
  // Simple Stickman Logic

  glPushMatrix();
  glTranslatef(pos.x, pos.y - 0.2f, pos.z); // Neck/Shoulder level generally

  // Body Spine
  glColor3f(0.5f, 0.5f, 0.5f);
  glBegin(GL_LINES);
  glVertex3f(0.0f, 0.0f, 0.0f);  // Neck
  glVertex3f(0.0f, -0.5f, 0.0f); // Hip
  // Shoulders
  glVertex3f(-0.2f, 0.0f, 0.0f);
  glVertex3f(0.2f, 0.0f, 0.0f);
  glEnd();

  // Arms Logic
  float time = (float)glfwGetTime();
  float t_pose = debug_appState.test_mode
                     ? debug_appState.t_pose_blend
                     : 1.0f; // Default T-Pose if not test mode? No, assume
                             // simple T-Pose base.
  float wave = debug_appState.test_mode && debug_appState.wave_anim
                   ? (sin(time * 5.0f) * 0.5f + 0.5f)
                   : 0.0f;

  // Left Arm
  // Shoulder (-0.2, 0, 0)
  // Elbow: Neural = (-0.25, -0.3, 0), T-Pose = (-0.5, 0, 0)
  // Lerp
  float elbow_lx = -0.25f + (-0.25f * t_pose);
  float elbow_ly = -0.3f + (0.3f * t_pose);

  // Wave Override (Left Arm raises)
  if (wave > 0.0f) {
    elbow_lx = -0.4f;
    elbow_ly = 0.2f + (wave * 0.2f);
  }

  // Draw Left Arm
  glColor3f(0.0f, 0.8f, 1.0f);
  glBegin(GL_LINES);
  glVertex3f(-0.2f, 0.0f, 0.0f);        // Shoulder
  glVertex3f(elbow_lx, elbow_ly, 0.0f); // Elbow
  // Forearm -> Hand
  glVertex3f(elbow_lx, elbow_ly, 0.0f);
  glVertex3f(elbow_lx - 0.25f, elbow_ly, 0.0f); // Hand
  glEnd();

  // Right Arm (Mirror)
  float elbow_rx = 0.25f + (0.25f * t_pose);
  float elbow_ry = -0.3f + (0.3f * t_pose);
  // Draw Right Arm
  glBegin(GL_LINES);
  glVertex3f(0.2f, 0.0f, 0.0f);         // Shoulder
  glVertex3f(elbow_rx, elbow_ry, 0.0f); // Elbow
  // Forearm
  glVertex3f(elbow_rx, elbow_ry, 0.0f);
  glVertex3f(elbow_rx + 0.25f, elbow_ry, 0.0f); // Hand (Pos)
  glEnd();

  // HANDS Curls
  // Draw sphere at hand pos, color by curl
  float l_curl = debug_appState.test_mode ? debug_appState.hand_l_curl : 0.0f;
  float r_curl = debug_appState.test_mode ? debug_appState.hand_r_curl : 0.0f;

  // L Hand
  glPointSize(
      15.0f *
      (0.5f +
       l_curl * 0.5f)); // Bigger = Fist? Or smaller? Let's say Color Change.
  glBegin(GL_POINTS);
  glColor3f(1.0f - l_curl, 1.0f, 1.0f - l_curl); // White -> Green
  glVertex3f(elbow_lx - 0.25f, elbow_ly, 0.0f);
  glEnd();

  // R Hand
  glPointSize(15.0f * (0.5f + r_curl * 0.5f));
  glBegin(GL_POINTS);
  glColor3f(1.0f - r_curl, 1.0f, 1.0f - r_curl); // White -> Green
  glVertex3f(elbow_rx + 0.25f, elbow_ry, 0.0f);
  glEnd();

  glPopMatrix();

  glBindFramebuffer(GL_FRAMEBUFFER, 0);
}

// --- MAIN ---
int main() {
  std::cout << "[Core] Lancement de VRChat Universal Video Bridge..."
            << std::endl;

  // Detect available cameras
  std::cout << "[Core] Détection des caméras disponibles..." << std::endl;
  for (int i = 0; i < 5; i++) {
    cv::VideoCapture test_cap(i);
    if (test_cap.isOpened()) {
      g_appState.camera_available[i] = true;
      test_cap.release();
      std::cout << "  ✓ Caméra " << i << " : Disponible" << std::endl;
    } else {
      g_appState.camera_available[i] = false;
      std::cout << "  ✗ Caméra " << i << " : Non disponible" << std::endl;
    }
  }

  // 1. GLFW Init
  if (!glfwInit())
    return 1;

  const char *glsl_version = "#version 130";
  glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 3);
  glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 0);
  glfwWindowHint(GLFW_SCALE_TO_MONITOR, GLFW_TRUE);

  GLFWwindow *window = glfwCreateWindow(
      1600, 900, "VRChat Video Bridge V19.1 (Pro)", NULL, NULL);
  if (!window)
    return 1;

  glfwMakeContextCurrent(window);
  glfwSwapInterval(1); // VSync

  if (!gladLoadGLLoader((GLADloadproc)glfwGetProcAddress)) {
    return 1;
  }

  // 2. ImGui Init
  IMGUI_CHECKVERSION();
  ImGui::CreateContext();
  ImGuiIO &io = ImGui::GetIO();
  (void)io;
  io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;
  io.ConfigFlags |= ImGuiConfigFlags_DockingEnable;

  if (std::filesystem::exists("C:\\Windows\\Fonts\\segoeui.ttf")) {
    io.Fonts->AddFontFromFileTTF("C:\\Windows\\Fonts\\segoeui.ttf", 26.0f);
  } else {
    io.Fonts->AddFontDefault();
    io.FontGlobalScale = 1.5f;
  }

  ImGui_ImplGlfw_InitForOpenGL(window, true);
  ImGui_ImplOpenGL3_Init(glsl_version);

  // 3. Start Vision Thread
  std::thread vision_thread(VisionLoop);

  // Wait a moment for WebServer to start and populate IP
  std::this_thread::sleep_for(std::chrono::seconds(1));

  std::string display_ip = "127.0.0.1";
  int display_port = 8080;
  {
    std::lock_guard<std::mutex> lock(g_appState.mutex);
    display_ip = g_appState.server_ip;
    display_port = g_appState.server_port;
  }

  UI::MainWindow main_window(display_ip, display_port);

  // --- Start Cloudflare Tunnel ASYNCHRONOUSLY (non-blocking) ---
  // Create tunnel instance globally to avoid destruction
  static Network::CloudflareTunnel tunnel;

  std::cout << "[Network] Starting Cloudflare Tunnel in background..."
            << std::endl;

  // Launch tunnel in separate thread to avoid blocking UI
  std::thread tunnel_thread([]() {
    // Wait for WebServer to be fully ready before starting tunnel
    // This prevents Cloudflare Error 1033 (can't connect to backend)
    std::this_thread::sleep_for(std::chrono::seconds(3));

    if (tunnel.Start(8080)) {
      std::string tunnel_url = tunnel.GetPublicURL();
      std::string github_pages_url =
          "https://shazamifius.github.io/VRchatFACE-HAND-ARM-tracking";
      std::string full_qr_url = github_pages_url + "?tunnel=" + tunnel_url;

      std::cout << "[Network] ============================================"
                << std::endl;
      std::cout << "[Network] Phone Link URL: " << full_qr_url << std::endl;
      std::cout << "[Network] ============================================"
                << std::endl;

      // Instead of calling UpdateQRCode directly (not thread-safe),
      // store in shared state for UI thread to pick up
      {
        std::lock_guard<std::mutex> lock(g_appState.mutex);
        g_appState.tunnel_url = full_qr_url;
        g_appState.tunnel_ready = true;
      }
    } else {
      std::cerr << "[Network] WARNING: Cloudflare Tunnel failed to start."
                << std::endl;
      std::cerr << "[Network] Phone Link will only work on local network."
                << std::endl;
    }
  });
  tunnel_thread.detach(); // Let it run independently

  OffscreenBuffer avatar_buffer;
  avatar_buffer.Init(512, 512);

  // 4. Main Event Loop (UI)
  GLuint camera_tex_id = 0;
  Core::Vector3 current_head_pos = {0, 0, 0};
  Core::Quaternion current_head_rot = {1, 0, 0, 0};
  UI::BodyState current_body_debug;

  while (!glfwWindowShouldClose(window)) {
    glfwPollEvents();

    // Sync Data
    {
      std::lock_guard<std::mutex> lock(g_appState.mutex);
      if (g_appState.new_frame_ready) {
        UpdateGLTexture(g_appState.latest_frame, camera_tex_id);
        g_appState.new_frame_ready = false;
      }
      current_head_pos = g_appState.head_pos;
      current_head_rot = g_appState.head_rot;

      // Update our local copy of debug state for rendering
      // BUT we also need to write the UI changes BACK to shared state so logic
      // can see it (if logic needed it) Actually, since UI modifies its own
      // state, we should Sync UI changes -> SharedState. But MainWindow.Render
      // modifies 'current_body_debug' if passed by ref. So we should:
      // 1. Read Shared -> Local (to get latest External updates if any)
      // 2. Render (UI modifies Local)
      // 3. Write Local -> Shared

      // However, SharedState.body_debug is mostly 'read' by VisionLoop (if we
      // implemented that logic) and 'written' by UI. So we can just take the
      // UI's version as authoritative for Test Mode. Let's just keep
      // 'current_body_debug' persistent in main, and update g_appState with it.
      g_appState.body_debug = current_body_debug;
    }

    // Render 3D Avatar to FBO (using the debug state)
    RenderSkeleton(avatar_buffer, current_head_pos, current_head_rot,
                   current_body_debug);

    // Render ImGui
    ImGui_ImplOpenGL3_NewFrame();
    ImGui_ImplGlfw_NewFrame();
    ImGui::NewFrame();

    // Camera Controls
    // Move Forward (Z or W for AZERTY/QWERTY support)
    if (ImGui::IsKeyDown(ImGuiKey_Z) || ImGui::IsKeyDown(ImGuiKey_W))
      g_camera.distance -= 0.05f; // Zoom in / Forward

    // Move Backward (S)
    if (ImGui::IsKeyDown(ImGuiKey_S))
      g_camera.distance += 0.05f; // Zoom out / Backward

    // Rotate Camera (A / E)
    if (ImGui::IsKeyDown(ImGuiKey_A))
      g_camera.angle_x -= 1.0f; // Rotate Left
    if (ImGui::IsKeyDown(ImGuiKey_E))
      g_camera.angle_x += 1.0f; // Rotate Right

    // Reset (R)
    if (ImGui::IsKeyDown(ImGuiKey_R))
      g_camera.Reset();

    // Look Around (Arrow Keys)
    if (ImGui::IsKeyDown(ImGuiKey_LeftArrow))
      g_camera.angle_x -= 1.0f;
    if (ImGui::IsKeyDown(ImGuiKey_RightArrow))
      g_camera.angle_x += 1.0f;
    if (ImGui::IsKeyDown(ImGuiKey_UpArrow))
      g_camera.angle_y -= 1.0f; // Tilt Up
    if (ImGui::IsKeyDown(ImGuiKey_DownArrow))
      g_camera.angle_y += 1.0f; // Tilt Down

    int display_w, display_h;
    glfwGetFramebufferSize(window, &display_w, &display_h);

    // Pass both textures AND debug state to UI
    // Calculate Stats
    float real_fps = ImGui::GetIO().Framerate;
    long long latency_val = 0;
    {
      std::lock_guard<std::mutex> lock(g_appState.mutex);
      latency_val = g_appState.inference_time_us;
    }

    int requested_cam = -1;
    // Copy camera availability status
    bool cam_status[5];
    {
      std::lock_guard<std::mutex> lock(g_appState.mutex);
      for (int i = 0; i < 5; i++) {
        cam_status[i] = g_appState.camera_available[i];
      }
    }

    main_window.Render(camera_tex_id, avatar_buffer.texture, display_w,
                       display_h, current_body_debug, real_fps, latency_val,
                       requested_cam, cam_status);

    // Check if UI requested camera change
    if (requested_cam != -1) {
      std::lock_guard<std::mutex> lock(g_appState.mutex);
      g_appState.pending_camera_index = requested_cam;
    }

    ImGui::Render();
    glViewport(0, 0, display_w, display_h);
    glClearColor(0, 0, 0, 1);
    glClear(GL_COLOR_BUFFER_BIT);
    ImGui_ImplOpenGL3_RenderDrawData(ImGui::GetDrawData());

    glfwSwapBuffers(window);
  }

  // Shutdown
  {
    std::lock_guard<std::mutex> lock(g_appState.mutex);
    g_appState.system_running = false;
  }
  if (vision_thread.joinable()) {
    vision_thread.join();
  }

  // Cleanup GL resources
  if (camera_tex_id)
    glDeleteTextures(1, &camera_tex_id);
  glDeleteFramebuffers(1, &avatar_buffer.fbo);
  glDeleteTextures(1, &avatar_buffer.texture);
  glDeleteRenderbuffers(1, &avatar_buffer.rbo);

  ImGui_ImplOpenGL3_Shutdown();
  ImGui_ImplGlfw_Shutdown();
  ImGui::DestroyContext();
  glfwDestroyWindow(window);
  glfwTerminate();

  return 0;
}
