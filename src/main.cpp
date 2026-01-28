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
};

SharedState g_appState;
CameraState g_camera; // 3D camera controls

// --- VISION THREAD ---
void VisionLoop() {
  std::cout << "[Thread] Vision Thread Started." << std::endl;

  // Init components locally to thread
  Vision::InferenceEngine ai_engine;
  ai_engine.LoadModel(L"models/yolov8n-pose.onnx");

  // NEW: Face Mesh for facial expressions
  Vision::FaceMesh face_engine;
  face_engine.LoadModel(
      L"models/face_landmarker.onnx"); // Will use STUB if not found
  Biomech::BlendshapeCalculator blendshape_calc;

  Vision::ModelSelector quality_selector;
  Biomech::CoordinateConverter converter;
  Network::OSCClient osc_client("127.0.0.1", 9000); // Port 9000 standard VRChat

  cv::VideoCapture cap(0);
  if (!cap.isOpened()) {
    std::cerr << "[Error] Camera not found in Vision Thread!" << std::endl;
  }

  while (true) {
    // Check exit condition
    {
      std::lock_guard<std::mutex> lock(g_appState.mutex);
      if (!g_appState.system_running)
        break;
    }

    // Check for Camera Switch
    {
      std::lock_guard<std::mutex> lock(g_appState.mutex);
      if (g_appState.pending_camera_index != -1) {
        std::cout << "[Thread] Switching to Camera "
                  << g_appState.pending_camera_index << std::endl;
        cap.open(g_appState.pending_camera_index);
        if (!cap.isOpened()) {
          std::cerr << "[Error] Could not open Camera "
                    << g_appState.pending_camera_index << std::endl;
        }
        g_appState.pending_camera_index = -1;
      }
    }

    cv::Mat frame;
    if (cap.isOpened()) {
      cap >> frame;
    }

    if (!frame.empty()) {
      auto start_inf = std::chrono::high_resolution_clock::now();

      // 1. Run Inference and get pose
      Vision::PoseResult pose;
      bool pose_detected = ai_engine.RunInference(frame, pose);

      auto end_inf = std::chrono::high_resolution_clock::now();
      long long duration =
          std::chrono::duration_cast<std::chrono::microseconds>(end_inf -
                                                                start_inf)
              .count();

      // 1b. Run Face Mesh inference (for blendshapes)
      auto start_face = std::chrono::high_resolution_clock::now();
      Vision::FaceMeshResult face_result;
      bool face_detected = face_engine.RunInference(frame, face_result);

      // Calculate blendshapes from face landmarks
      Biomech::ARKitBlendshapes blendshapes;
      if (face_detected && face_result.IsValid()) {
        blendshapes = blendshape_calc.Calculate(face_result);
      }

      auto end_face = std::chrono::high_resolution_clock::now();
      long long face_duration =
          std::chrono::duration_cast<std::chrono::microseconds>(end_face -
                                                                start_face)
              .count();

      // 2. Convert Pose to Skeleton Positions
      Core::Vector3 ai_head_pos(0.0f, 1.6f, 0.5f); // Default
      Core::Quaternion ai_head_rot(1, 0, 0, 0);

      if (pose_detected && pose.IsValid()) {
        // Convert 2D keypoints to 3D positions
        // Normalize nose position (center of frame = origin)
        float norm_x = (pose.Nose().x / frame.cols) - 0.5f;
        float norm_y = (pose.Nose().y / frame.rows) - 0.5f;

        // Head position (fix Y inversion)
        ai_head_pos.x = norm_x * 2.0f; // X: -1 to 1
        ai_head_pos.y =
            1.7f + norm_y * 1.5f; // Y: 1.7 ± movement (FIXED: was -)
        ai_head_pos.z = 0.5f;     // Z: fixed for now

        // Estimate head rotation from eye positions
        if (pose.LeftEye().IsValid() && pose.RightEye().IsValid()) {
          float eye_dx = pose.RightEye().x - pose.LeftEye().x;
          float eye_dy = pose.RightEye().y - pose.LeftEye().y;
          float roll_angle = atan2(eye_dy, eye_dx);

          // Simple roll rotation (around Z axis)
          ai_head_rot.w = cos(roll_angle / 2.0f);
          ai_head_rot.x = 0.0f;
          ai_head_rot.y = 0.0f;
          ai_head_rot.z = sin(roll_angle / 2.0f);
        }
      } else {
        // Fallback to mockup if no detection
        float time = (float)glfwGetTime();
        ai_head_pos.x = sin(time) * 0.2f;
        ai_head_pos.y = 1.6f + cos(time) * 0.05f;
        ai_head_pos.z = 0.5f;
      }

      // 3. OSC
      Core::Vector3 unity_head_pos = converter.ConvertPosition(ai_head_pos);
      Core::Quaternion unity_head_rot = converter.ConvertRotation(ai_head_rot);

      // Send to VRChat
      osc_client.Send(Network::VMCProtocol::PackBonePos("Head", unity_head_pos,
                                                        unity_head_rot));

      // Send Blendshapes (VRCFaceTracking v2 format)
      if (face_detected && face_result.IsValid()) {
        // Send essential blendshapes to VRChat
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
        // TODO: Send all other blendshapes as needed
      }

      // 4. Update Shared State
      {
        std::lock_guard<std::mutex> lock(g_appState.mutex);
        g_appState.latest_frame = frame.clone(); // Deep copy for UI
        g_appState.new_frame_ready = true;
        g_appState.head_pos = unity_head_pos;
        g_appState.head_rot = unity_head_rot;
        g_appState.inference_time_us = duration;
        g_appState.face_inference_time_us = face_duration; // NEW
        g_appState.blendshapes = blendshapes;              // NEW
        g_appState.face_tracking_active =
            face_detected && face_result.IsValid(); // NEW
      }
    } else {
      std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
  }

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

  // --- FLOOR GRID ---
  glBegin(GL_LINES);
  glColor3f(0.2f, 0.2f, 0.3f);
  for (int i = -10; i <= 10; ++i) {
    glVertex3f((float)i, 0.0f, -10.0f);
    glVertex3f((float)i, 0.0f, 10.0f);
    glVertex3f(-10.0f, 0.0f, (float)i);
    glVertex3f(10.0f, 0.0f, (float)i);
  }
  glEnd();

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

  GLFWwindow *window =
      glfwCreateWindow(1600, 900, "VRChat Video Bridge (Pro)", NULL, NULL);
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

  UI::MainWindow main_window;
  OffscreenBuffer avatar_buffer;
  avatar_buffer.Init(512, 512);

  // 3. Start Vision Thread
  std::thread vision_thread(VisionLoop);

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

    // Camera Controls (WASDQER keys)
    if (ImGui::IsKeyDown(ImGuiKey_W))
      g_camera.distance -= 0.05f; // Zoom in
    if (ImGui::IsKeyDown(ImGuiKey_S))
      g_camera.distance += 0.05f; // Zoom out
    if (ImGui::IsKeyDown(ImGuiKey_A))
      g_camera.angle_x -= 1.0f; // Rotate left
    if (ImGui::IsKeyDown(ImGuiKey_D))
      g_camera.angle_x += 1.0f; // Rotate right
    if (ImGui::IsKeyDown(ImGuiKey_Q))
      g_camera.height += 0.05f; // Move up
    if (ImGui::IsKeyDown(ImGuiKey_E))
      g_camera.height -= 0.05f; // Move down
    if (ImGui::IsKeyDown(ImGuiKey_R))
      g_camera.Reset(); // Reset camera

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
