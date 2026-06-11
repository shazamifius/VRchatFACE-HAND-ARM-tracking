//! Desktop-style control of the two virtual VR controllers exposed by the
//! `vrcbridge` driver. This is what lets the user navigate VRChat's VR menus,
//! click to log in / pick a world, and walk around — using only mouse +
//! keyboard, reproducing the desktop VRChat feel through emulated VR hardware.
//!
//! Control scheme (v1 — expect to tune sensitivity/bindings from feedback):
//!   - Mouse (no button held): aims the RIGHT-hand laser pointer for menus.
//!   - Left mouse button:      RIGHT trigger (select / click UI).
//!   - Hold RIGHT mouse button: head-look (handled by [`super::mouse_look`]);
//!                              the laser aim freezes while looking around.
//!   - WASD:                   LEFT thumbstick (locomotion).
//!   - Tab:                    menu (RIGHT B click).
//!   - Space:                  jump (RIGHT A click).
//!
//! Transport: one packed 45-byte packet per hand to UDP 127.0.0.1:39572,
//! matching `ControllerInputPacket` in driver/src/driver_main.cpp.

use anyhow::Result;
use nalgebra::{Quaternion, UnitQuaternion, Vector3};
use std::net::UdpSocket;

// Button bits — must match `enum ControllerButton` in the C++ driver.
const BTN_TRIGGER: u32 = 1 << 0;
const BTN_A: u32 = 1 << 1;
const BTN_B: u32 = 1 << 2;
#[allow(dead_code)]
const BTN_SYSTEM: u32 = 1 << 3;
#[allow(dead_code)]
const BTN_THUMBSTICK: u32 = 1 << 4;
#[allow(dead_code)]
const BTN_GRIP: u32 = 1 << 5;

/// Minimal Win32 bindings for global keyboard state.
#[cfg(windows)]
mod sys {
    #[link(name = "user32")]
    extern "system" {
        pub fn GetAsyncKeyState(v_key: i32) -> i16;
        pub fn MapVirtualKeyW(u_code: u32, u_map_type: u32) -> u32;
    }

    pub const VK_LBUTTON: i32 = 0x01;
    pub const VK_TAB: i32 = 0x09;
    pub const VK_SPACE: i32 = 0x20;

    // Physical scancodes (Set 1) for the WASD movement cluster. We resolve them
    // to virtual keys via the ACTIVE layout, so the same physical keys work on
    // AZERTY (ZQSD), QWERTZ, etc. — not just QWERTY.
    pub const SC_W: u32 = 0x11; // forward  (AZERTY: Z)
    pub const SC_A: u32 = 0x1E; // left     (AZERTY: Q)
    pub const SC_S: u32 = 0x1F; // back     (AZERTY: S)
    pub const SC_D: u32 = 0x20; // right    (AZERTY: D)
    const MAPVK_VSC_TO_VK: u32 = 1;

    pub unsafe fn key_down(vk: i32) -> bool {
        (GetAsyncKeyState(vk) as u16 & 0x8000) != 0
    }

    /// Is the physical key at `scancode` (a Set-1 scancode) currently down,
    /// regardless of keyboard layout?
    pub unsafe fn key_down_sc(scancode: u32) -> bool {
        let vk = MapVirtualKeyW(scancode, MAPVK_VSC_TO_VK);
        if vk == 0 {
            return false;
        }
        (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0
    }
}

/// One hand's full state, ready to serialize.
struct HandState {
    hand: u8,
    pos: [f32; 3],
    quat_xyzw: [f32; 4],
    buttons: u32,
    trigger: f32,
    thumb: [f32; 2],
}

pub struct ControllerInput {
    socket: UdpSocket,
    target: String,
}

impl ControllerInput {
    const ADDR: &'static str = "127.0.0.1:39572";
    /// Eye height the head sits at (matches HeadBridge / driver default).
    const HEAD_Y: f32 = 1.6;

    pub fn new() -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        Ok(Self {
            socket,
            target: Self::ADDR.to_string(),
        })
    }

    /// Poll mouse+keyboard and drive both hands. `head_quat_xyzw` is the current
    /// fused head orientation (mouse-look + webcam). The laser points along it —
    /// a "gaze reticle": you aim by turning your head with the mouse, and left
    /// click selects. This is far more predictable than mapping the cursor into
    /// a 3D laser, which felt scrambled.
    #[cfg(windows)]
    pub fn update(&self, head_quat_xyzw: [f32; 4]) -> Result<()> {
        let q_head = UnitQuaternion::from_quaternion(Quaternion::new(
            head_quat_xyzw[3],
            head_quat_xyzw[0],
            head_quat_xyzw[1],
            head_quat_xyzw[2],
        ));

        // --- Buttons / axes ---
        let (lmb, tab, space, tx, ty) = unsafe {
            let lmb = sys::key_down(sys::VK_LBUTTON);
            let tab = sys::key_down(sys::VK_TAB);
            let space = sys::key_down(sys::VK_SPACE);
            let mut tx = 0.0f32;
            let mut ty = 0.0f32;
            // Physical WASD cluster (AZERTY users get ZQSD automatically).
            if sys::key_down_sc(sys::SC_A) {
                tx -= 1.0;
            }
            if sys::key_down_sc(sys::SC_D) {
                tx += 1.0;
            }
            if sys::key_down_sc(sys::SC_W) {
                ty += 1.0;
            }
            if sys::key_down_sc(sys::SC_S) {
                ty -= 1.0;
            }
            (lmb, tab, space, tx, ty)
        };

        let mut right_buttons = 0u32;
        if lmb {
            right_buttons |= BTN_TRIGGER;
        }
        if tab {
            right_buttons |= BTN_B; // VRChat main menu
        }
        if space {
            right_buttons |= BTN_A; // jump
        }

        // Hands at realistic human proportions in front of the body: shoulder
        // width apart (~0.4 m) and well below the head (~chest/waist). VRChat's
        // VR avatar scaling can size you from the head<->hands span, and hands
        // bunched up at the head made the avatar tiny (~10 cm). They still point
        // along the gaze so the right-hand laser stays a center reticle.
        let head_pos = Vector3::new(0.0, Self::HEAD_Y, 0.0);
        let right_pos = head_pos + q_head * Vector3::new(0.20, -0.45, -0.35);
        let left_pos = head_pos + q_head * Vector3::new(-0.20, -0.45, -0.35);

        let q_right = q_head;
        let q_left = q_head;

        let right = HandState {
            hand: 1,
            pos: [right_pos.x, right_pos.y, right_pos.z],
            quat_xyzw: quat_to_xyzw(&q_right),
            buttons: right_buttons,
            trigger: if lmb { 1.0 } else { 0.0 },
            thumb: [0.0, 0.0],
        };
        let left = HandState {
            hand: 0,
            pos: [left_pos.x, left_pos.y, left_pos.z],
            quat_xyzw: quat_to_xyzw(&q_left),
            buttons: 0,
            trigger: 0.0,
            thumb: [tx, ty],
        };

        self.send(&left)?;
        self.send(&right)?;
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn update(&self, _head_quat_xyzw: [f32; 4]) -> Result<()> {
        Ok(())
    }

    fn send(&self, h: &HandState) -> Result<()> {
        // 45-byte packed layout: hand(u8) pos(3f) quat(4f) buttons(u32) trig(f) thumb(2f)
        let mut buf = [0u8; 45];
        buf[0] = h.hand;
        let mut o = 1;
        let put = |buf: &mut [u8; 45], o: &mut usize, v: f32| {
            buf[*o..*o + 4].copy_from_slice(&v.to_le_bytes());
            *o += 4;
        };
        for v in h.pos {
            put(&mut buf, &mut o, v);
        }
        for v in h.quat_xyzw {
            put(&mut buf, &mut o, v);
        }
        buf[o..o + 4].copy_from_slice(&h.buttons.to_le_bytes());
        o += 4;
        put(&mut buf, &mut o, h.trigger);
        for v in h.thumb {
            put(&mut buf, &mut o, v);
        }
        self.socket.send_to(&buf, &self.target)?;
        Ok(())
    }
}

fn quat_to_xyzw(q: &UnitQuaternion<f32>) -> [f32; 4] {
    let q = q.quaternion();
    [q.i, q.j, q.k, q.w]
}
