//! Global **mouse-look** for the virtual head: turns raw OS cursor motion into
//! accumulated yaw/pitch so the user can rotate a full 360° (the webcam tops
//! out near ±86° because the face leaves the frame).
//!
//! Classic FPS capture: while the **right mouse button** is held, we read the
//! cursor delta, accumulate it into yaw/pitch, and re-center the cursor to its
//! press anchor so it can never reach a screen edge. Releasing the button frees
//! the cursor again for normal desktop use.
//!
//! The accumulated angles are *fused* with the webcam orientation in the logic
//! thread (mouse yaw about world-up, mouse pitch about local-right, composed in
//! front of the webcam quaternion), giving 360° turning + natural head motion.

/// Minimal Win32 bindings — we only need three calls, so we declare them
/// directly instead of pulling in the whole `windows` crate.
#[cfg(windows)]
mod sys {
    #[repr(C)]
    pub struct POINT {
        pub x: i32,
        pub y: i32,
    }

    #[link(name = "user32")]
    extern "system" {
        pub fn GetAsyncKeyState(v_key: i32) -> i16;
        pub fn GetCursorPos(point: *mut POINT) -> i32;
        pub fn SetCursorPos(x: i32, y: i32) -> i32;
    }

    /// Right mouse button virtual-key code.
    pub const VK_RBUTTON: i32 = 0x02;
}

pub struct MouseLook {
    /// Accumulated yaw (radians, about world-up). Unbounded — wraps for 360°.
    yaw: f32,
    /// Accumulated pitch (radians, about local-right). Clamped to ±85°.
    pitch: f32,
    /// Radians of rotation per pixel of cursor movement.
    sensitivity: f32,
    /// Whether the right button was held last poll (so a fresh press doesn't
    /// produce a spurious delta from the previous cursor location).
    active: bool,
    anchor_x: i32,
    anchor_y: i32,
}

impl Default for MouseLook {
    fn default() -> Self {
        Self::new()
    }
}

impl MouseLook {
    /// ~0.0025 rad/px ≈ a full turn in ~2500 px of drag — comfortable default.
    const SENSITIVITY: f32 = 0.0025;
    /// Don't let the head pitch past near-vertical.
    const PITCH_LIMIT_RAD: f32 = 1.483_530; // 85°

    pub fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            sensitivity: Self::SENSITIVITY,
            active: false,
            anchor_x: 0,
            anchor_y: 0,
        }
    }

    /// Poll the OS mouse once. Call every logic frame. Returns the accumulated
    /// `(yaw, pitch)` in radians. While the right button is held the cursor is
    /// re-centered so it stays captured; otherwise the cursor is left free.
    #[cfg(windows)]
    pub fn poll(&mut self) -> (f32, f32) {
        unsafe {
            // High bit set => key is currently down.
            let held = (sys::GetAsyncKeyState(sys::VK_RBUTTON) as u16 & 0x8000) != 0;
            if held {
                let mut p = sys::POINT { x: 0, y: 0 };
                if sys::GetCursorPos(&mut p) != 0 {
                    if !self.active {
                        // Just pressed: anchor here, emit no delta this frame.
                        self.active = true;
                        self.anchor_x = p.x;
                        self.anchor_y = p.y;
                    } else {
                        let dx = (p.x - self.anchor_x) as f32;
                        let dy = (p.y - self.anchor_y) as f32;
                        self.yaw += dx * self.sensitivity;
                        // Mouse up (dy < 0) looks up.
                        self.pitch += dy * self.sensitivity;
                        self.pitch = self
                            .pitch
                            .clamp(-Self::PITCH_LIMIT_RAD, Self::PITCH_LIMIT_RAD);
                        // Re-center to the anchor so the cursor never escapes and
                        // the next frame's delta is measured from the same point.
                        sys::SetCursorPos(self.anchor_x, self.anchor_y);
                    }
                }
            } else {
                self.active = false;
            }
        }
        (self.yaw, self.pitch)
    }

    /// Non-Windows stub: no global cursor capture available, so mouse-look is a
    /// no-op (the webcam still drives the head).
    #[cfg(not(windows))]
    pub fn poll(&mut self) -> (f32, f32) {
        (self.yaw, self.pitch)
    }
}
