#pragma once

#include <string>
#include <vector>

#include <imgui.h>
#include <imgui_impl_glfw.h>
#include <imgui_impl_opengl3.h>

// Forward decl
struct GLFWwindow;

namespace UI {

// Shared body state for Debugging/Visualization
struct BodyState {
  // Face
  float jaw_open = 0.0f;
  float blink_l = 0.0f;
  float blink_r = 0.0f;
  float tongue_out = 0.0f;

  // Hands (0.0 = Open, 1.0 = Closed)
  float hand_l_curl = 0.0f;
  float hand_r_curl = 0.0f;

  // Arms / Body Control
  bool test_mode = false;    // If true, ignore AI and use these values
  float t_pose_blend = 1.0f; // 1.0 = T-Pose, 0.0 = Neutral
  bool wave_anim = false;    // Simple wave animation
};

class MainWindow {
public:
  MainWindow() {
    GenerateQRPlaceholder();
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

  void Render(unsigned int camera_texture_id,
              unsigned int avatar_preview_texture_id, int width, int height,
              BodyState &body_debug, float fps, long long latency_us,
              int &requested_camera_index, const bool camera_available[5]) {
    // Custom Background
    DrawBackground(width, height);

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
      if (ImGui::Button("AUTO-CONFIG VRCHAT", ImVec2(-1, 35))) {
        // TODO: Trigger auto config
      }
    }

    ImGui::Spacing();
    if (ImGui::CollapsingHeader("DEBUG / TEST MODE",
                                ImGuiTreeNodeFlags_DefaultOpen)) {
      ImGui::Checkbox("ENABLE TEST MODE", &body_debug.test_mode);

      if (body_debug.test_mode) {
        ImGui::Indent();
        ImGui::TextColored(ImVec4(0.0f, 0.8f, 1.0f, 1.0f), "Face Controls");
        ImGui::SliderFloat("Jaw Open", &body_debug.jaw_open, 0.0f, 1.0f);
        ImGui::SliderFloat("Blink (L)", &body_debug.blink_l, 0.0f, 1.0f);
        ImGui::SliderFloat("Blink (R)", &body_debug.blink_r, 0.0f, 1.0f);
        ImGui::SliderFloat("Tongue", &body_debug.tongue_out, 0.0f, 1.0f);

        ImGui::Spacing();
        ImGui::TextColored(ImVec4(0.0f, 0.8f, 1.0f, 1.0f), "Hand Controls");
        ImGui::SliderFloat("L Hand Curl", &body_debug.hand_l_curl, 0.0f, 1.0f);
        ImGui::SliderFloat("R Hand Curl", &body_debug.hand_r_curl, 0.0f, 1.0f);

        ImGui::Spacing();
        ImGui::TextColored(ImVec4(0.0f, 0.8f, 1.0f, 1.0f), "Body Controls");
        ImGui::SliderFloat("T-Pose Blend", &body_debug.t_pose_blend, 0.0f,
                           1.0f);
        ImGui::Checkbox("Wave Animation", &body_debug.wave_anim);

        ImGui::Unindent();
      } else {
        ImGui::TextDisabled("Enable Test Mode to manually\\ncontrol skeleton.");
      }
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

    if (qr_texture_id_ != 0) {
      ImGui::Image((ImTextureID)(intptr_t)qr_texture_id_,
                   ImVec2(qr_size, qr_size), ImVec2(0, 0), ImVec2(1, 1),
                   ImVec4(1, 1, 1, 1), ImVec4(0, 1, 0.8f, 0.5f)); // Cyan border
    }
    ImGui::End();

    // --- Main View (Center) ---
    // We want tabs for different views
    ImGui::Begin("VISUALIZER", nullptr, ImGuiWindowFlags_NoCollapse);
    if (ImGui::BeginTabBar("MainTabs")) {
      if (ImGui::BeginTabItem("LIVE PREVIEW (3D)")) {
        // 3D Avatar Preview
        ImVec2 avail = ImGui::GetContentRegionAvail();
        if (avatar_preview_texture_id != 0) {
          // Keep aspect ratio
          float aspect = (float)width / (float)height * 0.5f; // Rough estimate
          // We just stretch to fit for now, 3D render is usually square-ish
          ImGui::Image((ImTextureID)(intptr_t)avatar_preview_texture_id, avail);
        } else {
          ImGui::Text("Waiting for 3D Renderer...");
        }
        ImGui::EndTabItem();
      }
      if (ImGui::BeginTabItem("CAMERA FEED")) {
        // Raw Camera Feed
        ImVec2 avail = ImGui::GetContentRegionAvail();
        if (camera_texture_id != 0) {
          ImGui::Image((ImTextureID)(intptr_t)camera_texture_id, avail);
        } else {
          ImGui::TextDisabled("NO SIGNAL");
        }
        ImGui::EndTabItem();
      }
      ImGui::EndTabBar();
    }
    ImGui::End();
  }

private:
  GLuint qr_texture_id_ = 0;

  void GenerateQRPlaceholder() {
    // Generate a cool "Cyber" QR code pattern (placeholder)
    const int w = 64;
    const int h = 64;
    std::vector<unsigned char> pixels(w * h * 4);
    for (int i = 0; i < w * h; ++i) {
      int r = (rand() % 255) > 200 ? 255 : 0; // High contrast noise
      pixels[i * 4 + 0] = r ? 0 : 0;
      pixels[i * 4 + 1] = r ? 255 : 0; // Cyan/Green type
      pixels[i * 4 + 2] = r ? 255 : 0;
      pixels[i * 4 + 3] = 255;
    }
    // ... (We would reuse the GL generation code properly or helper)
    // For brevity in this tool call, I'm assuming existing helper or simple
    // recreation
    glGenTextures(1, &qr_texture_id_);
    glBindTexture(GL_TEXTURE_2D, qr_texture_id_);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, w, h, 0, GL_RGBA, GL_UNSIGNED_BYTE,
                 pixels.data());
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
