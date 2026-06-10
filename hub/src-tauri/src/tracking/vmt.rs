use anyhow::Result;
use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::UdpSocket;

use crate::tracking::solver::TrackerData;

/// Output backend that drives **VirtualMotionTracker (VMT)** virtual SteamVR
/// devices over OSC/UDP.
///
/// This is the "sensor emulation" path: instead of pushing limited VRChat
/// avatar parameters (which can't move the head/body in desktop mode), we send
/// device *poses* to VMT, a SteamVR driver that registers them as REAL tracked
/// devices. VRChat's full-body IK then animates the avatar from them — torso
/// lean, hips, legs — exactly what OSC avatar params can't do.
///
/// VMT listens on UDP 127.0.0.1:39570 and accepts (among others):
///   /VMT/Room/Unity  (int index, int enable, float timeoffset,
///                     float x,y,z,  float qx,qy,qz,qw)
/// in Unity room-space coordinates (left-handed, metres, relative to the
/// SteamVR play-space origin). VMT device roles are tracker(1)/controller(2,3)/
/// reference(4) — it has NO HMD role, so head rotation is a later phase via our
/// own HMD driver. Here we only drive body trackers/controllers.
///
/// Note on coordinate space: we forward `TrackerData` poses as-is. Aligning the
/// webcam space to the VRChat play space (scale, floor height, facing) is the
/// calibration step handled in the solver — this module is just the transport.
pub struct VmtBridge {
    socket: UdpSocket,
    target: String,
}

impl VmtBridge {
    /// VMT's fixed OSC receive endpoint.
    const VMT_ADDR: &'static str = "127.0.0.1:39570";

    pub fn new() -> Result<Self> {
        // Ephemeral local port; VMT only ever receives from us.
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        Ok(Self {
            socket,
            target: Self::VMT_ADDR.to_string(),
        })
    }

    /// Drive one virtual tracker at `index` to the given Unity room-space pose.
    /// `enable` registers/keeps the device alive (1 = tracker role).
    pub fn send_pose(
        &self,
        index: i32,
        pos: [f32; 3],
        quat_xyzw: [f32; 4],
    ) -> Result<()> {
        let msg = OscMessage {
            addr: "/VMT/Room/Unity".to_string(),
            args: vec![
                OscType::Int(index),
                OscType::Int(1), // enable = tracker role
                OscType::Float(0.0), // timeoffset (s)
                OscType::Float(pos[0]),
                OscType::Float(pos[1]),
                OscType::Float(pos[2]),
                OscType::Float(quat_xyzw[0]),
                OscType::Float(quat_xyzw[1]),
                OscType::Float(quat_xyzw[2]),
                OscType::Float(quat_xyzw[3]),
            ],
        };
        let buf = encoder::encode(&OscPacket::Message(msg))?;
        self.socket.send_to(&buf, &self.target)?;
        Ok(())
    }

    /// Drive a full set of body trackers in one go. The tracker `id` is used
    /// directly as the VMT device index, so the caller owns the id↔role map
    /// (e.g. 1=hip, 2=chest, 3=left foot, 4=right foot).
    pub fn send_trackers(&self, trackers: &[TrackerData]) -> Result<()> {
        for t in trackers {
            self.send_pose(t.id, t.position, t.rotation)?;
        }
        Ok(())
    }

    /// Integration self-test: move tracker `index` in a slow horizontal circle
    /// of `radius` metres at `t` seconds, identity rotation. Used to verify the
    /// whole transport (Rust → VMT → SteamVR) end-to-end before any AI is wired
    /// in — if this device shows up moving in SteamVR, the pipeline is proven.
    pub fn send_test_circle(&self, index: i32, t: f32, radius: f32) -> Result<()> {
        let x = radius * t.cos();
        let z = radius * t.sin();
        // ~1 m off the floor so it's visible in the SteamVR room view.
        self.send_pose(index, [x, 1.0, z], [0.0, 0.0, 0.0, 1.0])
    }
}
