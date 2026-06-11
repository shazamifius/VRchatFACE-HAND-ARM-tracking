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

/// Minimal Win32 bindings for global key/mouse/screen state.
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
        pub fn GetSystemMetrics(index: i32) -> i32;
    }

    pub const VK_LBUTTON: i32 = 0x01;
    pub const VK_TAB: i32 = 0x09;
    pub const VK_SPACE: i32 = 0x20;
    pub const VK_W: i32 = 0x57;
    pub const VK_A: i32 = 0x41;
    pub const VK_S: i32 = 0x53;
    pub const VK_D: i32 = 0x44;
    pub const SM_CXSCREEN: i32 = 0;
    pub const SM_CYSCREEN: i32 = 1;

    pub unsafe fn key_down(vk: i32) -> bool {
        (GetAsyncKeyState(vk) as u16 & 0x8000) != 0
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
    /// Max laser sweep from center, radians (mouse at screen edge).
    const AIM_YAW_RANGE: f32 = 0.7; // ~40°
    const AIM_PITCH_RANGE: f32 = 0.5; // ~29°

    pub fn new() -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        Ok(Self {
            socket,
            target: Self::ADDR.to_string(),
        })
    }

    /// Poll mouse+keyboard and drive both hands. `head_quat_xyzw` is the current
    /// fused head orientation, so the hands and laser stay anchored in front of
    /// wherever the user is looking. `rmb_held` suppresses laser re-aim while the
    /// user is head-looking (the cursor is captured then).
    #[cfg(windows)]
    pub fn update(&self, head_quat_xyzw: [f32; 4], rmb_held: bool) -> Result<()> {
        let q_head = UnitQuaternion::from_quaternion(Quaternion::new(
            head_quat_xyzw[3],
            head_quat_xyzw[0],
            head_quat_xyzw[1],
            head_quat_xyzw[2],
        ));

        // --- Mouse aim -> laser offset from head orientation ---
        let (aim_yaw, aim_pitch) = if rmb_held {
            (0.0, 0.0) // head-look mode: keep laser pointing straight ahead
        } else {
            unsafe {
                let sw = sys::GetSystemMetrics(sys::SM_CXSCREEN).max(1) as f32;
                let sh = sys::GetSystemMetrics(sys::SM_CYSCREEN).max(1) as f32;
                let mut p = sys::POINT { x: 0, y: 0 };
                if sys::GetCursorPos(&mut p) != 0 {
                    let nx = (p.x as f32 / sw) * 2.0 - 1.0; // [-1,1], left->right
                    let ny = (p.y as f32 / sh) * 2.0 - 1.0; // [-1,1], top->bottom
                    (-nx * Self::AIM_YAW_RANGE, ny * Self::AIM_PITCH_RANGE)
                } else {
                    (0.0, 0.0)
                }
            }
        };
        let q_aim = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), aim_yaw)
            * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), aim_pitch);

        // --- Buttons / axes ---
        let (lmb, tab, space, tx, ty) = unsafe {
            let lmb = sys::key_down(sys::VK_LBUTTON);
            let tab = sys::key_down(sys::VK_TAB);
            let space = sys::key_down(sys::VK_SPACE);
            let mut tx = 0.0f32;
            let mut ty = 0.0f32;
            if sys::key_down(sys::VK_A) {
                tx -= 1.0;
            }
            if sys::key_down(sys::VK_D) {
                tx += 1.0;
            }
            if sys::key_down(sys::VK_W) {
                ty += 1.0;
            }
            if sys::key_down(sys::VK_S) {
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

        // Hand world poses: a fixed local offset rotated into the head frame so
        // the hands sit in front of the user and turn with the view.
        let right_local = Vector3::new(0.2, -0.3, -0.3);
        let left_local = Vector3::new(-0.2, -0.3, -0.3);
        let head_pos = Vector3::new(0.0, Self::HEAD_Y, 0.0);
        let right_pos = head_pos + q_head * right_local;
        let left_pos = head_pos + q_head * left_local;

        let q_right = q_head * q_aim;
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
    pub fn update(&self, _head_quat_xyzw: [f32; 4], _rmb_held: bool) -> Result<()> {
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
