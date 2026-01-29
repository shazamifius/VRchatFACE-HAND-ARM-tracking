#pragma once

#include <string>
#include <vector>

#include <imgui.h>
#include <imgui_impl_glfw.h>
#include <imgui_impl_opengl3.h>
#include <qrencode.h>

#include "biomech/UserCalibration.hpp"

// Forward decl
struct GLFWwindow;

namespace UI {

// BodyState struct removed as it was for 3D puppet control

class MainWindow {
public:
  MainWindow(const std::string &ip, int port) {
    std::string url = "http://" + ip + ":" + std::to_string(port);
    GenerateQRPlaceholder(url);
    SetupStyle();
  }

  // Draw a gradient background for the whole window
  void DrawBackground(int width, int height) {
    ImDrawList *draw_list = ImGui::GetBackgroundDrawList();
    ImU32 col_top = ImGui::GetColorU32(ImVec4(0.02f, 0.02f, 0.05f, 1.0f));
    ImU32 col_bot = ImGui::GetColorU32(ImVec4(0.05f, 0.05f, 0.10f, 1.0f));
    draw_list->AddRectFilledMultiColor(ImVec2(0, 0),
                                       ImVec2((float)width, (float)height),
                                       col_top, col_top, col_bot, col_bot);
  }

  void Render(unsigned int camera_texture_id, int width, int height, float fps,
              long long latency_us, int &requested_camera_index,
              const bool camera_available[5],
              Biomech::CalibrationCommand &calibration_cmd,
              const std::string &feedback_msg, int calib_state_int,
              int calib_samples_collected, int calib_total_samples,
              double countdown_remaining, bool is_loading, bool &trigger_rescan,
              bool phone_connected, const std::string &phone_info) {
    // Custom Background
    DrawBackground(width, height);

    // ... (rest of function until Wizard) ...

    // DockSpace
    ImGui::DockSpaceOverViewport(0, ImGui::GetMainViewport(),
                                 ImGuiDockNodeFlags_PassthruCentralNode);

    // --- Sidebar (Left) ---
    ImGui::Begin("CONTROL CENTER", nullptr, ImGuiWindowFlags_NoCollapse);
    ImGui::TextColored(ImVec4(0.0f, 1.0f, 0.8f, 1.0f), "STATUS: ONLINE");
    ImGui::Separator();
    ImGui::Spacing();

    // Connection Info
    ImGui::PushStyleColor(ImGuiCol_Header, ImVec4(0.1f, 0.1f, 0.15f, 1.0f));
    if (ImGui::CollapsingHeader("CONNECTIVITY",
                                ImGuiTreeNodeFlags_DefaultOpen)) {
      ImGui::Text("Target IP:");
      ImGui::SameLine();
      ImGui::TextColored(ImVec4(0.7f, 0.7f, 0.7f, 1.0f), "127.0.0.1");
      ImGui::Text("OSC Port:");
      ImGui::SameLine();
      ImGui::TextColored(ImVec4(0.7f, 0.7f, 0.7f, 1.0f), "9000");

      ImGui::Spacing();

      // Camera Selection with Status Indicators
      ImGui::Text("Select Camera");
      static int current_cam = 0;

      // Phone Camera Selection
      ImGui::Spacing();
      {
        ImVec4 dot_color = phone_connected ? ImVec4(0.0f, 1.0f, 0.0f, 1.0f)
                                           : ImVec4(1.0f, 0.0f, 0.0f, 1.0f);
        ImGui::PushStyleColor(ImGuiCol_Text, dot_color);
        ImGui::Text("●");
        ImGui::PopStyleColor();
        ImGui::SameLine();

        if (current_cam == 99) {
          ImGui::PushStyleColor(ImGuiCol_Button,
                                ImVec4(0.0f, 0.6f, 0.7f, 1.0f));
        }
        if (ImGui::Button("Phone Camera (Network)", ImVec2(-1, 0))) {
          current_cam = 99;
          requested_camera_index = 99;
        }
        if (current_cam == 99) {
          ImGui::PopStyleColor();
        }
      }
      ImGui::Spacing();
      ImGui::Separator();
      ImGui::Spacing();

      for (int i = 0; i < 5; i++) {
        // Status indicator (colored dot)
        ImVec4 dot_color = camera_available[i] ? ImVec4(0.0f, 1.0f, 0.0f, 1.0f)
                                               :               // Green
                               ImVec4(1.0f, 0.0f, 0.0f, 1.0f); // Red
        ImGui::PushStyleColor(ImGuiCol_Text, dot_color);
        ImGui::Text("●");
        ImGui::PopStyleColor();

        ImGui::SameLine();

        // Camera button
        char label[32];
        snprintf(label, sizeof(label), "Camera %d%s", i,
                 camera_available[i] ? "" : " (Indisponible)");

        if (current_cam == i) {
          ImGui::PushStyleColor(ImGuiCol_Button,
                                ImVec4(0.0f, 0.6f, 0.7f, 1.0f));
        }

        if (ImGui::Button(label, ImVec2(-1, 0))) {
          if (camera_available[i]) {
            current_cam = i;
            requested_camera_index = i;
          }
        }

        if (current_cam == i) {
          ImGui::PopStyleColor();
        }
      }

      ImGui::Spacing();
      // Rescan Button (Small, less prominent)
      if (ImGui::Button("↻ Rescan Cameras")) {
        trigger_rescan = true;
      }
      if (ImGui::IsItemHovered())
        ImGui::SetTooltip("Refresh camera list if you plugged in a new device");

      ImGui::Spacing();

      // Phone Connection Status
      ImGui::Text("Phone Connection");

      ImVec4 phone_dot_color = phone_connected ? ImVec4(0.0f, 1.0f, 0.0f, 1.0f)
                                               : ImVec4(1.0f, 0.0f, 0.0f, 1.0f);
      ImGui::PushStyleColor(ImGuiCol_Text, phone_dot_color);
      ImGui::Text("●");
      ImGui::PopStyleColor();
      ImGui::SameLine();
      ImGui::TextColored(ImVec4(0.7f, 0.7f, 0.7f, 1.0f),
                         phone_connected ? phone_info.c_str() : "Disconnected");

      ImGui::Spacing();
      // Auto-config removed placeholder logic for now
    }

    ImGui::Spacing();
    ImGui::Separator();

    // --- CALIBRATION WIZARD ---
    if (ImGui::CollapsingHeader("FACE CALIBRATION",
                                ImGuiTreeNodeFlags_DefaultOpen)) {
      ImGui::TextColored(ImVec4(1.0f, 0.8f, 0.2f, 1.0f),
                         "[ Calibration Wizard ]");
      ImGui::TextWrapped("Click buttons while holding the expression.");

      ImGui::Spacing();
      // 1. NEUTRAL
      if (ImGui::Button("1. Capture NEUTRAL", ImVec2(-1, 0))) {
        calibration_cmd = Biomech::CalibrationCommand::NEUTRAL;
      }
      if (ImGui::IsItemHovered())
        ImGui::SetTooltip("Relax your face completely. Look forward.");

      ImGui::Spacing();
      // 2. SMILE
      if (ImGui::Button("2. Capture SMILE", ImVec2(-1, 0))) {
        calibration_cmd = Biomech::CalibrationCommand::SMILE;
      }
      if (ImGui::IsItemHovered())
        ImGui::SetTooltip("Smile as wide as you can!");

      ImGui::Spacing();
      // 3. EYES CLOSED
      if (ImGui::Button("3. Capture EYES CLOSED", ImVec2(-1, 0))) {
        calibration_cmd = Biomech::CalibrationCommand::BLINK_CLOSED;
      }
      if (ImGui::IsItemHovered())
        ImGui::SetTooltip("Close your eyes completely.");

      ImGui::Spacing();
      ImGui::Separator();

      // SAVE
      ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.0f, 0.5f, 0.2f, 1.0f));
      ImGui::PushStyleColor(ImGuiCol_ButtonHovered,
                            ImVec4(0.0f, 0.7f, 0.3f, 1.0f));
      if (ImGui::Button("SAVE PROFILE", ImVec2(-1, 0))) {
        calibration_cmd = Biomech::CalibrationCommand::SAVE;
      }
      ImGui::PopStyleColor(2);
    }

    ImGui::Spacing();
    if (ImGui::CollapsingHeader("PERFORMANCE",
                                ImGuiTreeNodeFlags_DefaultOpen)) {
      ImGui::Text("Latency");
      std::string lat_str = std::to_string(latency_us / 1000) + "ms";
      float lat_fraction = (float)latency_us / 33000.0f; // 33ms target
      if (lat_fraction > 1.0f)
        lat_fraction = 1.0f;
      ImGui::ProgressBar(lat_fraction, ImVec2(-1, 0), lat_str.c_str());

      ImGui::Text("FPS");
      std::string fps_str = std::to_string((int)fps) + " fps";
      float fps_fraction = fps / 60.0f;
      if (fps_fraction > 1.0f)
        fps_fraction = 1.0f;
      ImGui::ProgressBar(fps_fraction, ImVec2(-1, 0), fps_str.c_str());
    }
    ImGui::PopStyleColor();

    // Spacer
    ImGui::Dummy(ImVec2(0, 20));
    ImGui::Separator();
    ImGui::Dummy(ImVec2(0, 5));

    // Mobile Link
    ImGui::TextColored(ImVec4(0.0f, 0.8f, 1.0f, 1.0f), "MOBILE LINK");
    float qr_size = ImGui::GetContentRegionAvail().x;
    if (qr_size > 180)
      qr_size = 180;

    // Center QR
    float avail_x = ImGui::GetContentRegionAvail().x;
    ImGui::SetCursorPosX(ImGui::GetCursorPosX() + (avail_x - qr_size) * 0.5f);

    if (is_loading) {
      ImGui::SetCursorPosX(ImGui::GetCursorPosX() + (avail_x - qr_size) * 0.5f);
      // Placeholder box
      ImGui::PushStyleColor(ImGuiCol_ChildBg, ImVec4(0, 0, 0, 0.5f));
      ImGui::BeginChild("QRLoading", ImVec2(qr_size, qr_size), true);

      // Center loading text
      std::string load_txt = "Starting Tunnel...";
      ImVec2 txt_sz = ImGui::CalcTextSize(load_txt.c_str());
      ImGui::SetCursorPos(
          ImVec2((qr_size - txt_sz.x) * 0.5f, (qr_size - txt_sz.y) * 0.5f));
      ImGui::TextColored(ImVec4(1, 0.8f, 0, 1), "%s", load_txt.c_str());

      ImGui::EndChild();
      ImGui::PopStyleColor();
    } else if (qr_texture_id_ != 0) {
      ImGui::Image((ImTextureID)(intptr_t)qr_texture_id_,
                   ImVec2(qr_size, qr_size), ImVec2(0, 0), ImVec2(1, 1),
                   ImVec4(1, 1, 1, 1), ImVec4(0, 1, 0.8f, 0.5f)); // Cyan border
    }
    ImGui::End();

    // --- Main View (Center) ---
    // Single View: Camera Feed
    ImGui::Begin("VISUALIZER", nullptr, ImGuiWindowFlags_NoCollapse);
    ImVec2 avail = ImGui::GetContentRegionAvail();
    ImVec2 window_pos = ImGui::GetWindowPos();
    ImVec2 content_offset = ImGui::GetCursorScreenPos();

    if (camera_texture_id != 0) {
      // Maintain aspect ratio if possible, or fill?
      // Usually users want to see the whole image.
      // Image is often 16:9 or 4:3.
      // avail.x / avail.y vs img definitions.
      // Let's just fit width and adjust height, or fit to avail.
      ImGui::Image((ImTextureID)(intptr_t)camera_texture_id, avail);
    } else {
      // Centered Text
      ImVec2 text_size = ImGui::CalcTextSize("NO SIGNAL");
      ImGui::SetCursorPos(ImVec2((avail.x - text_size.x) * 0.5f,
                                 (avail.y - text_size.y) * 0.5f));
      ImGui::TextDisabled("NO SIGNAL");
    }

    // --- CALIBRATION OVERLAYS ---
    ImDrawList *draw_list = ImGui::GetWindowDrawList();

    // 0=IDLE, 1=COUNTDOWN, 2=SAMPLING
    if (calib_state_int == 1) {
      // COUNTDOWN OVERLAY
      int sec_left = (int)std::ceil(countdown_remaining);
      if (sec_left < 1)
        sec_left = 1;
      std::string text = "HOLD POSE: " + std::to_string(sec_left);

      // Calculate centered position
      ImFont *font = ImGui::GetFont();
      float font_size = 72.0f;
      ImVec2 text_size =
          font->CalcTextSizeA(font_size, FLT_MAX, 0.0f, text.c_str());
      ImVec2 text_pos =
          ImVec2(content_offset.x + (avail.x - text_size.x) * 0.5f,
                 content_offset.y + (avail.y - text_size.y) * 0.5f);

      // Draw semi-transparent background
      ImVec2 bg_padding(20, 15);
      draw_list->AddRectFilled(
          ImVec2(text_pos.x - bg_padding.x, text_pos.y - bg_padding.y),
          ImVec2(text_pos.x + text_size.x + bg_padding.x,
                 text_pos.y + text_size.y + bg_padding.y),
          IM_COL32(0, 0, 0, 180), 10.0f);

      // Draw text with glow effect
      draw_list->AddText(font, font_size,
                         ImVec2(text_pos.x + 2, text_pos.y + 2),
                         IM_COL32(0, 0, 0, 200), text.c_str());
      draw_list->AddText(font, font_size, text_pos, IM_COL32(255, 200, 0, 255),
                         text.c_str());
    } else if (calib_state_int == 2) {
      // SAMPLING OVERLAY
      std::string text = "SAMPLING... " +
                         std::to_string(calib_samples_collected) + "/" +
                         std::to_string(calib_total_samples);

      ImFont *font = ImGui::GetFont();
      float font_size = 48.0f;
      ImVec2 text_size =
          font->CalcTextSizeA(font_size, FLT_MAX, 0.0f, text.c_str());
      ImVec2 text_pos =
          ImVec2(content_offset.x + (avail.x - text_size.x) * 0.5f,
                 content_offset.y + (avail.y - text_size.y) * 0.5f);

      // Background
      ImVec2 bg_padding(20, 15);
      draw_list->AddRectFilled(
          ImVec2(text_pos.x - bg_padding.x, text_pos.y - bg_padding.y),
          ImVec2(text_pos.x + text_size.x + bg_padding.x,
                 text_pos.y + text_size.y + bg_padding.y),
          IM_COL32(0, 0, 0, 180), 10.0f);

      // Text
      draw_list->AddText(font, font_size,
                         ImVec2(text_pos.x + 2, text_pos.y + 2),
                         IM_COL32(0, 0, 0, 200), text.c_str());
      draw_list->AddText(font, font_size, text_pos, IM_COL32(0, 255, 200, 255),
                         text.c_str());

      // Progress bar
      float progress =
          (float)calib_samples_collected / (float)calib_total_samples;
      float bar_width = 300.0f;
      float bar_height = 8.0f;
      ImVec2 bar_pos = ImVec2(content_offset.x + (avail.x - bar_width) * 0.5f,
                              text_pos.y + text_size.y + 30.0f);

      // Background bar
      draw_list->AddRectFilled(
          bar_pos, ImVec2(bar_pos.x + bar_width, bar_pos.y + bar_height),
          IM_COL32(50, 50, 50, 200), 4.0f);

      // Progress bar
      draw_list->AddRectFilled(
          bar_pos,
          ImVec2(bar_pos.x + bar_width * progress, bar_pos.y + bar_height),
          IM_COL32(0, 255, 200, 255), 4.0f);
    }

    // SUCCESS OVERLAY (when feedback contains "Success" or "Saved")
    if (feedback_msg.find("Success") != std::string::npos ||
        feedback_msg.find("Saved") != std::string::npos) {
      std::string text = "✓ " + feedback_msg;

      ImFont *font = ImGui::GetFont();
      float font_size = 56.0f;
      ImVec2 text_size =
          font->CalcTextSizeA(font_size, FLT_MAX, 0.0f, text.c_str());
      ImVec2 text_pos =
          ImVec2(content_offset.x + (avail.x - text_size.x) * 0.5f,
                 content_offset.y + avail.y * 0.3f);

      // Background
      ImVec2 bg_padding(25, 18);
      draw_list->AddRectFilled(
          ImVec2(text_pos.x - bg_padding.x, text_pos.y - bg_padding.y),
          ImVec2(text_pos.x + text_size.x + bg_padding.x,
                 text_pos.y + text_size.y + bg_padding.y),
          IM_COL32(0, 100, 0, 200), 10.0f);

      // Text
      draw_list->AddText(font, font_size,
                         ImVec2(text_pos.x + 2, text_pos.y + 2),
                         IM_COL32(0, 0, 0, 200), text.c_str());
      draw_list->AddText(font, font_size, text_pos, IM_COL32(0, 255, 100, 255),
                         text.c_str());
    }

    ImGui::End();
  }

private:
  GLuint qr_texture_id_ = 0;
  std::string current_qr_url_;

public:
  // Update QR code with new URL (for Cloudflare Tunnel)
  void UpdateQRCode(const std::string &full_url) {
    current_qr_url_ = full_url;
    GenerateQRPlaceholder(full_url);
  }

private:
  void GenerateQRPlaceholder(const std::string &url) {

    // Generate QR with libqrencode
    // Signature: QRcode_encodeString(string, version, level, hint,
    // casesensitive)
    QRcode *qr =
        QRcode_encodeString(url.c_str(), 0, QR_ECLEVEL_M, QR_MODE_8, 1);
    if (!qr) {
      return; // Failed to generate QR
    }

    int border = 4;
    int qr_size = qr->width;
    int size = qr_size + border * 2;
    std::vector<unsigned char> pixels(size * size * 4);

    // Colors
    unsigned char fg[4] = {0, 255, 200, 255}; // Cyan/Green Neon
    unsigned char bg[4] = {20, 20, 30, 255};  // Dark background

    for (int y = 0; y < size; y++) {
      for (int x = 0; x < size; x++) {
        // Check if inside QR area
        bool isDark = false;
        int qx = x - border;
        int qy = y - border;

        if (qx >= 0 && qx < qr_size && qy >= 0 && qy < qr_size) {
          // libqrencode stores data row-major: data[y * width + x]
          isDark = (qr->data[qy * qr_size + qx] & 1);
        }

        int idx = (y * size + x) * 4;
        if (isDark) {
          pixels[idx + 0] = fg[0];
          pixels[idx + 1] = fg[1];
          pixels[idx + 2] = fg[2];
          pixels[idx + 3] = fg[3];
        } else {
          pixels[idx + 0] = bg[0];
          pixels[idx + 1] = bg[1];
          pixels[idx + 2] = bg[2];
          pixels[idx + 3] = bg[3];
        }
      }
    }

    // Free QR code
    QRcode_free(qr);

    if (qr_texture_id_ != 0)
      glDeleteTextures(1, &qr_texture_id_);

    glGenTextures(1, &qr_texture_id_);
    glBindTexture(GL_TEXTURE_2D, qr_texture_id_);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, size, size, 0, GL_RGBA,
                 GL_UNSIGNED_BYTE, pixels.data());
  }

  void SetupStyle() {
    ImGuiStyle &style = ImGui::GetStyle();
    ImGui::StyleColorsDark();

    // ROUNDINGS
    style.WindowRounding = 8.0f;
    style.ChildRounding = 8.0f;
    style.FrameRounding = 5.0f;
    style.GrabRounding = 5.0f;
    style.PopupRounding = 8.0f;
    style.ScrollbarRounding = 12.0f;
    style.TabRounding = 8.0f;

    // PADDING
    style.WindowPadding = ImVec2(10, 10);
    style.FramePadding = ImVec2(10, 6);
    style.ItemSpacing = ImVec2(10, 10);

    // COLORS (Cyberpunk / Neon Future)
    ImVec4 *colors = style.Colors;
    colors[ImGuiCol_Text] = ImVec4(0.90f, 0.90f, 0.95f, 1.00f);
    colors[ImGuiCol_TextDisabled] = ImVec4(0.50f, 0.50f, 0.50f, 1.00f);
    colors[ImGuiCol_WindowBg] = ImVec4(0.08f, 0.08f, 0.10f, 0.96f);
    colors[ImGuiCol_ChildBg] = ImVec4(0.00f, 0.00f, 0.00f, 0.00f);
    colors[ImGuiCol_PopupBg] = ImVec4(0.12f, 0.12f, 0.14f, 0.95f);
    colors[ImGuiCol_Border] = ImVec4(0.20f, 0.20f, 0.25f, 0.50f);
    colors[ImGuiCol_BorderShadow] = ImVec4(0.00f, 0.00f, 0.00f, 0.00f);

    // HEADERS (Cyan/Purple Accents)
    colors[ImGuiCol_FrameBg] = ImVec4(0.15f, 0.15f, 0.20f, 1.00f);
    colors[ImGuiCol_FrameBgHovered] = ImVec4(0.20f, 0.20f, 0.25f, 1.00f);
    colors[ImGuiCol_FrameBgActive] = ImVec4(0.25f, 0.25f, 0.35f, 1.00f);

    colors[ImGuiCol_TitleBg] = ImVec4(0.06f, 0.06f, 0.08f, 1.00f);
    colors[ImGuiCol_TitleBgActive] = ImVec4(0.06f, 0.06f, 0.08f, 1.00f);

    // TABS & INTERACTION
    colors[ImGuiCol_Tab] = ImVec4(0.10f, 0.10f, 0.15f, 1.00f);
    colors[ImGuiCol_TabHovered] =
        ImVec4(0.00f, 0.80f, 0.90f, 0.60f); // Neon Cyan Hover
    colors[ImGuiCol_TabActive] =
        ImVec4(0.00f, 0.60f, 0.70f, 1.00f); // Neon Cyan Active
    colors[ImGuiCol_TabUnfocused] = ImVec4(0.10f, 0.10f, 0.15f, 1.00f);
    colors[ImGuiCol_TabUnfocusedActive] = ImVec4(0.15f, 0.15f, 0.20f, 1.00f);

    colors[ImGuiCol_Button] = ImVec4(0.20f, 0.20f, 0.30f, 0.80f);
    colors[ImGuiCol_ButtonHovered] = ImVec4(0.00f, 0.60f, 0.80f, 1.00f);
    colors[ImGuiCol_ButtonActive] = ImVec4(0.00f, 0.70f, 0.90f, 1.00f);

    colors[ImGuiCol_Header] = ImVec4(0.20f, 0.20f, 0.30f, 1.00f);
    colors[ImGuiCol_HeaderHovered] = ImVec4(0.30f, 0.30f, 0.45f, 1.00f);
    colors[ImGuiCol_HeaderActive] = ImVec4(0.40f, 0.40f, 0.55f, 1.00f);

    colors[ImGuiCol_Separator] = ImVec4(0.30f, 0.30f, 0.40f, 0.50f);
    colors[ImGuiCol_SeparatorHovered] = ImVec4(0.40f, 0.40f, 0.50f, 0.78f);
    colors[ImGuiCol_SeparatorActive] = ImVec4(0.50f, 0.50f, 0.60f, 1.00f);

    colors[ImGuiCol_ResizeGrip] = ImVec4(0.00f, 0.70f, 0.90f, 0.20f);
    colors[ImGuiCol_ResizeGripHovered] = ImVec4(0.00f, 0.70f, 0.90f, 0.67f);
    colors[ImGuiCol_ResizeGripActive] = ImVec4(0.00f, 0.70f, 0.90f, 0.95f);

    colors[ImGuiCol_PlotLines] = ImVec4(0.00f, 0.90f, 1.00f, 1.00f);
    colors[ImGuiCol_PlotLinesHovered] = ImVec4(1.00f, 0.40f, 0.00f, 1.00f);
    colors[ImGuiCol_PlotHistogram] = ImVec4(0.00f, 0.90f, 1.00f, 1.00f);
    colors[ImGuiCol_PlotHistogramHovered] = ImVec4(1.00f, 0.40f, 0.00f, 1.00f);
  }
};

} // namespace UI
