//! Body pose -> VMT tracker mapping.
//!
//! Turns BlazePose's 33 image-space landmarks into a small set of full-body
//! SteamVR trackers (hips / chest / feet) in VMT "Unity room-space" metres.
//!
//! v1 mapping is deliberately simple and NOT yet metrically calibrated: we map
//! the image box the body occupies onto a ~1.8 m tall, ~1.5 m wide room volume.
//! The goal of this stage is to prove that the body drives the trackers (they
//! move in SteamVR with the person); true scale/floor/facing alignment is the
//! T-pose calibration step that comes next. Rotations are identity for now —
//! position movement is what proves the chain.

use crate::tracking::solver::TrackerData;

// BlazePose landmark indices we use (subject's own left/right).
const L_SHOULDER: usize = 11;
const R_SHOULDER: usize = 12;
const L_HIP: usize = 23;
const R_HIP: usize = 24;
const L_ANKLE: usize = 27;
const R_ANKLE: usize = 28;

// VMT device indices. The caller owns the id<->role map; these match the
// comment in vmt.rs (1=hip, 2=chest, 3=left foot, 4=right foot).
pub const VMT_HIP: i32 = 1;
pub const VMT_CHEST: i32 = 2;
pub const VMT_LEFT_FOOT: i32 = 3;
pub const VMT_RIGHT_FOOT: i32 = 4;

// Room volume the image box maps onto (metres).
const ROOM_HEIGHT: f32 = 1.8;
const ROOM_WIDTH: f32 = 1.5;

const IDENTITY_QUAT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// Map one image-space landmark (pixels) to a Unity room-space position.
/// - x: mirrored (selfie camera) and centred, scaled to ROOM_WIDTH.
/// - y: image-top -> high, image-bottom -> floor (0), scaled to ROOM_HEIGHT.
/// - z: 0 for v1 (depth from pose z is added with calibration later).
fn to_room(px: f32, py: f32, frame_w: f32, frame_h: f32) -> [f32; 3] {
    let nx = (px / frame_w) - 0.5;
    let x = -nx * ROOM_WIDTH; // mirror so the avatar leans the same way as the user
    let y = (1.0 - (py / frame_h)) * ROOM_HEIGHT;
    [x, y, 0.0]
}

fn midpoint(pose: &[[f32; 3]], a: usize, b: usize, frame_w: f32, frame_h: f32) -> [f32; 3] {
    let pa = to_room(pose[a][0], pose[a][1], frame_w, frame_h);
    let pb = to_room(pose[b][0], pose[b][1], frame_w, frame_h);
    [(pa[0] + pb[0]) * 0.5, (pa[1] + pb[1]) * 0.5, (pa[2] + pb[2]) * 0.5]
}

/// Build the VMT body trackers (hips, chest, both feet) from a 33-point pose.
/// Returns empty if the pose is too short to be a real BlazePose result.
pub fn pose_to_body_trackers(pose: &[[f32; 3]], frame_w: f32, frame_h: f32) -> Vec<TrackerData> {
    if pose.len() < 33 {
        return Vec::new();
    }
    let frame_w = if frame_w > 1.0 { frame_w } else { 640.0 };
    let frame_h = if frame_h > 1.0 { frame_h } else { 480.0 };

    let hips = midpoint(pose, L_HIP, R_HIP, frame_w, frame_h);
    let chest = midpoint(pose, L_SHOULDER, R_SHOULDER, frame_w, frame_h);
    let left_foot = to_room(pose[L_ANKLE][0], pose[L_ANKLE][1], frame_w, frame_h);
    let right_foot = to_room(pose[R_ANKLE][0], pose[R_ANKLE][1], frame_w, frame_h);

    vec![
        TrackerData { id: VMT_HIP, position: hips, rotation: IDENTITY_QUAT },
        TrackerData { id: VMT_CHEST, position: chest, rotation: IDENTITY_QUAT },
        TrackerData { id: VMT_LEFT_FOOT, position: left_foot, rotation: IDENTITY_QUAT },
        TrackerData { id: VMT_RIGHT_FOOT, position: right_foot, rotation: IDENTITY_QUAT },
    ]
}

/// A synthetic standing person (33 pts, 640x480 pixel space) that sways gently
/// from side to side at time `t` seconds. Used by the VMT_TEST=2 proof to drive
/// the REAL `pose_to_body_trackers` mapping without a camera, so the hips/chest/
/// feet trackers visibly move in SteamVR.
pub fn synthetic_standing_pose(t: f32) -> Vec<[f32; 3]> {
    let (fw, fh) = (640.0_f32, 480.0_f32);
    let cx = fw * 0.5 + (t.sin()) * fw * 0.12; // sway +/- ~12% of width
    let mut p = vec![[0.0_f32; 3]; 33];
    let set = |p: &mut Vec<[f32; 3]>, i: usize, x: f32, y: f32| { p[i] = [x, y, 0.0]; };

    // Shoulders (upper body sways most), hips (mid), ankles (planted-ish).
    set(&mut p, L_SHOULDER, cx + fw * 0.08, fh * 0.25);
    set(&mut p, R_SHOULDER, cx - fw * 0.08, fh * 0.25);
    set(&mut p, L_HIP, cx + fw * 0.05, fh * 0.55);
    set(&mut p, R_HIP, cx - fw * 0.05, fh * 0.55);
    // Feet barely sway (weight stays on the floor).
    let foot_cx = fw * 0.5 + (t.sin()) * fw * 0.02;
    set(&mut p, L_ANKLE, foot_cx + fw * 0.04, fh * 0.92);
    set(&mut p, R_ANKLE, foot_cx - fw * 0.04, fh * 0.92);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_trackers_are_anatomically_ordered() {
        let pose = synthetic_standing_pose(0.0);
        let trackers = pose_to_body_trackers(&pose, 640.0, 480.0);
        assert_eq!(trackers.len(), 4, "expected hips/chest/left+right foot");

        let get = |id: i32| trackers.iter().find(|t| t.id == id).unwrap().position;
        let hips = get(VMT_HIP);
        let chest = get(VMT_CHEST);
        let lfoot = get(VMT_LEFT_FOOT);
        let rfoot = get(VMT_RIGHT_FOOT);

        // Vertical order in room space: feet < hips < chest.
        assert!(lfoot[1] < hips[1] && rfoot[1] < hips[1], "feet must be below hips");
        assert!(hips[1] < chest[1], "hips must be below chest");
        // Everything sits within the room volume.
        for t in &trackers {
            assert!(t.position[1] >= 0.0 && t.position[1] <= ROOM_HEIGHT, "y in room");
        }
    }

    #[test]
    fn sway_moves_the_hips_horizontally() {
        let a = pose_to_body_trackers(&synthetic_standing_pose(0.0), 640.0, 480.0);
        let b = pose_to_body_trackers(&synthetic_standing_pose(std::f32::consts::FRAC_PI_2), 640.0, 480.0);
        let hip_a = a.iter().find(|t| t.id == VMT_HIP).unwrap().position[0];
        let hip_b = b.iter().find(|t| t.id == VMT_HIP).unwrap().position[0];
        assert!((hip_a - hip_b).abs() > 0.05, "hips should move horizontally as the body sways");
    }

    #[test]
    fn short_pose_yields_no_trackers() {
        assert!(pose_to_body_trackers(&vec![[0.0; 3]; 10], 640.0, 480.0).is_empty());
    }
}
