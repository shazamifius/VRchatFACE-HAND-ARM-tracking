use std::collections::HashMap;
use std::time::{Instant, Duration};


#[derive(Clone, Debug)]
pub struct UserProfile {
    // Face Offsets (Neutral = 0.0)
    pub neutral_face: HashMap<String, f32>,
    // Arm Lengths (cm)
    pub arm_upper_len: f32,
    pub arm_lower_len: f32,
    pub shoulder_width: f32, // Ratio to Head Width? or cm
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            neutral_face: HashMap::new(),
            arm_upper_len: 30.0,
            arm_lower_len: 25.0,
            shoulder_width: 30.0, // Center to Shoulder ~15cm? Total 30?
        }
    }
}

pub enum CalibrationStage {
    None,
    NeutralFace,
    TPose,
    CenterGaze,
}

pub struct CalibrationManager {
    pub stage: CalibrationStage,
    start_time: Option<Instant>,
    duration: Duration,
    samples: Vec<HashMap<String, f32>>, // For Face
    arm_samples: Vec<f32>, // For Arm Length (dist Shoulder->Hand)
}

impl CalibrationManager {
    pub fn new() -> Self {
        Self {
            stage: CalibrationStage::None,
            start_time: None,
            duration: Duration::from_secs(2),
            samples: Vec::new(),
            arm_samples: Vec::new(),
        }
    }

    pub fn start(&mut self, stage: CalibrationStage) {
        self.stage = stage;
        self.start_time = Some(Instant::now());
        self.samples.clear();
        self.arm_samples.clear();
        println!("[Calibration] Started {:?}", match self.stage {
            CalibrationStage::NeutralFace => "Neutral Face",
            CalibrationStage::TPose => "T-Pose",
            CalibrationStage::CenterGaze => "Center Gaze",
            _ => "None"
        });
    }

    /// Returns true if calibration is active
    pub fn is_calibrating(&self) -> bool {
        match self.stage {
            CalibrationStage::None => false,
            _ => true,
        }
    }

    /// Process calibration frame
    pub fn update(&mut self, params: &HashMap<String, f32>, arm_span: Option<f32>) -> Option<UserProfile> {
        if let Some(start) = self.start_time {
            if start.elapsed() > self.duration {
                // Finish Calibration
                return self.finish();
            }
            
            // Collect Samples
            match self.stage {
                CalibrationStage::NeutralFace | CalibrationStage::CenterGaze => {
                    self.samples.push(params.clone());
                },
                CalibrationStage::TPose => {
                    if let Some(dist) = arm_span {
                        self.arm_samples.push(dist);
                    }
                },
                _ => {}
            }
        }
        None
    }

    fn finish(&mut self) -> Option<UserProfile> {
        println!("[Calibration] Finished!");
        let mut profile = UserProfile::default(); 
        
        match self.stage {
            CalibrationStage::NeutralFace | CalibrationStage::CenterGaze => {
                // Average params
                let count = self.samples.len() as f32;
                if count > 0.0 {
                    let mut sums = HashMap::new();
                    // ... (Sum logic)
                    for s in &self.samples {
                        for (k, v) in s {
                            *sums.entry(k.clone()).or_insert(0.0) += v;
                        }
                    }
                    // Apply to profile.neutral_face
                    // Note: CenterGaze should probably update a DIFFERENT map?
                    // But for now, we merge into neutral_face.
                    // If we calibrate CenterGaze after NeutralFace, we overwrite?
                    // Ideally UserProfile should store distinct offsets.
                    // But for MVP, overwriting or merging into `neutral_face` is okay.
                    // Actually, if we calibrate Gaze, we only want to update Gaze keys.
                    // If `params` only has Jaw/Blink, then CenterGaze is redundant for now.
                    for (k, v) in sums {
                        profile.neutral_face.insert(k, v / count);
                    }
                }
            },
            CalibrationStage::TPose => {
                let count = self.arm_samples.len() as f32;
                if count > 0.0 {
                    let avg_dist = self.arm_samples.iter().sum::<f32>() / count;
                    profile.arm_upper_len = avg_dist * 0.55;
                    profile.arm_lower_len = avg_dist * 0.45;
                    println!("[Calibration] Arm Length: Total {:.1}cm -> Upper {:.1}, Lower {:.1}", 
                             avg_dist, profile.arm_upper_len, profile.arm_lower_len);
                }
            },
            _ => {}
        }
        
        self.stage = CalibrationStage::None;
        self.start_time = None;
        
        Some(profile)
    }
}
