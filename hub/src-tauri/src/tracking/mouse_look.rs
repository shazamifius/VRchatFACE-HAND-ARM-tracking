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

    /// Left Alt virtual-key code — held to RELEASE the captured cursor.
    pub const VK_LMENU: i32 = 0xA4;
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
    /// `(yaw, pitch)` in radians.
    ///
    /// Mouse-look is ACTIVE BY DEFAULT (the mouse turns the head like a normal
    /// PC game): each frame we read the cursor delta, accumulate it, and re-center
    /// the cursor so it never leaves the screen. **Hold Left Alt** to release the
    /// cursor for desktop use (clicking SteamVR buttons, alt-tab, etc.).
    #[cfg(windows)]
    pub fn poll(&mut self) -> (f32, f32) {
        unsafe {
            // Hold Left Alt to temporarily release the cursor.
            let released = (sys::GetAsyncKeyState(sys::VK_LMENU) as u16 & 0x8000) != 0;
            if released {
                self.active = false;
                return (self.yaw, self.pitch);
            }

            let mut p = sys::POINT { x: 0, y: 0 };
            if sys::GetCursorPos(&mut p) != 0 {
                if !self.active {
                    // Just (re)captured: anchor here, emit no delta this frame.
                    self.active = true;
                    self.anchor_x = p.x;
                    self.anchor_y = p.y;
                } else {
                    let dx = (p.x - self.anchor_x) as f32;
                    let dy = (p.y - self.anchor_y) as f32;
                    // Standard non-inverted FPS look: mouse right -> look right,
                    // mouse up -> look up.
                    self.yaw -= dx * self.sensitivity;
                    self.pitch -= dy * self.sensitivity;
                    self.pitch = self
                        .pitch
                        .clamp(-Self::PITCH_LIMIT_RAD, Self::PITCH_LIMIT_RAD);
                    // Re-center so the cursor never escapes and next frame's delta
                    // is measured from the same point.
                    sys::SetCursorPos(self.anchor_x, self.anchor_y);
                }
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

    /// Whether the mouse is currently captured for head-look (false while Left
    /// Alt is held to free the cursor). Used for diagnostics.
    pub fn is_capturing(&self) -> bool {
        self.active
    }
}
