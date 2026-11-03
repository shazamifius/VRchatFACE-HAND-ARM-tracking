use anyhow::Result;
use std::net::UdpSocket;

/// Output backend that drives our **own virtual HMD** (the `vrcbridge` SteamVR
/// driver) over a tiny UDP channel.
///
/// This is the head counterpart to [`crate::tracking::vmt::VmtBridge`]: VMT has
/// no HMD role, so head rotation goes to the C++ HMD driver we ship under
/// `driver/`. The driver listens on UDP 127.0.0.1:39571 and applies the pose to
/// the virtual headset every frame — that's what turns the VRChat view.
///
/// Packet = 7 little-endian `f32`: `qx, qy, qz, qw, px, py, pz`
///   - quaternion is **xyzw** (head orientation, world space)
///   - position is metres, **Y up**, relative to the SteamVR play-space floor
///
/// IMPORTANT — keep updates smooth and continuous. A large instantaneous jump in
/// position is read by SteamVR/VRChat as *tracking loss* and drops you to the
/// "waiting" overlay. The solver already low-pass filters the pose, so we just
/// forward it; we also lock eye height to a fixed standing value (see
/// [`HeadBridge::send_rotation`]) so the head never sinks to the floor.
pub struct HeadBridge {
    socket: UdpSocket,
    target: String,
}

impl HeadBridge {
    /// The vrcbridge HMD driver's fixed UDP receive endpoint (separate from
    /// VMT's 39570).
    const HEAD_ADDR: &'static str = "127.0.0.1:39571";

    /// Standing eye height in metres. Matches the user's VRChat avatar (~1.6 m)
    /// and the driver's own default, so rotation-only streaming never changes
    /// the height the headset sits at.
    pub const EYE_HEIGHT_M: f32 = 1.6;

    pub fn new() -> Result<Self> {
        // Ephemeral local port; the driver only ever receives from us.
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        Ok(Self {
            socket,
            target: Self::HEAD_ADDR.to_string(),
        })
    }

    /// Send a full head pose (orientation xyzw + position in metres, Y up).
    pub fn send_pose(&self, quat_xyzw: [f32; 4], pos_m: [f32; 3]) -> Result<()> {
        let mut buf = [0u8; 28];
        let floats = [
            quat_xyzw[0],
            quat_xyzw[1],
            quat_xyzw[2],
            quat_xyzw[3],
            pos_m[0],
            pos_m[1],
            pos_m[2],
        ];
        for (i, f) in floats.iter().enumerate() {
            buf[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
        }
        self.socket.send_to(&buf, &self.target)?;
        Ok(())
    }

    /// Send rotation only, holding the headset at a fixed standing eye height.
    ///
    /// This is the safe default for webcam head tracking: the webcam gives a
    /// reliable orientation but only a tiny, noisy head translation, and feeding
    /// that translation straight through drops the view toward the floor. So we
    /// pin position to `(0, EYE_HEIGHT_M, 0)` and only animate orientation.
    pub fn send_rotation(&self, quat_xyzw: [f32; 4]) -> Result<()> {
        self.send_pose(quat_xyzw, [0.0, Self::EYE_HEIGHT_M, 0.0])
    }
}
