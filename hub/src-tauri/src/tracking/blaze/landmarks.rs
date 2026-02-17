use anyhow::Result;
use ort::session::{Session, builder::GraphOptimizationLevel};
use image::{DynamicImage, GenericImageView};
use ndarray::Array4;
use crate::tracking::blaze::utils::{extract_roi, denormalize_landmarks};

pub struct BlazeLandmark {
    session: Session,
    input_size: (usize, usize), 
    num_landmarks: usize,
    num_dims: usize, // 2 or 3 (x, y, [z])
}

impl BlazeLandmark {
    pub fn new(model_path: &str, input_size: usize, num_landmarks: usize, num_dims: usize) -> Result<Self> {
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)?;

        Ok(Self {
            session,
            input_size: (input_size, input_size),
            num_landmarks,
            num_dims,
        })
    }

    /// Run inference on a specific region
    pub fn predict(
        &mut self, 
        image: &DynamicImage, 
        xc: f32, 
        yc: f32, 
        scale: f32, 
        theta: f32
    ) -> Result<(Vec<(f32, f32, f32)>, f32)> { // (Keypoints (x,y,z), Score)
        
        // 1. Extract ROI
        let roi_img = extract_roi(image, xc, yc, theta, scale, (self.input_size.0 as u32, self.input_size.1 as u32));

        // 2. Preprocess
        // Normalize 0..1 or -1..1?
        // BlazeLandmark usually expects 0..1
        // Let's check blazelandmark.py -> preprocess -> return x / 255.0
        // blazedetector was -1..1
        
        let mut input_tensor = Array4::<f32>::zeros((1, self.input_size.1, self.input_size.0, 3));
        for (x, y, pixel) in roi_img.pixels() {
            let r = pixel[0] as f32 / 255.0;
            let g = pixel[1] as f32 / 255.0;
            let b = pixel[2] as f32 / 255.0;
            input_tensor[[0, y as usize, x as usize, 0]] = r;
            input_tensor[[0, y as usize, x as usize, 1]] = g;
            input_tensor[[0, y as usize, x as usize, 2]] = b;
        }

        // 3. Inference
        let input_value = ort::value::Value::from_array(input_tensor)?;
        let inputs = ort::inputs!["input" => input_value];
        let outputs = self.session.run(inputs)?;

        // 4. Extract
        // Output indices:
        // Face Mesh: 
        // 0: landmarks (1, 1404) -> 468 * 3
        // 1: flag (1, 1) -> Score
        
        // Hand:
        // 0: landmarks (1, 63) -> 21 * 3
        // 1: flag (1, 1)
        // 2: handedness (1, 1)
        
        // We try to find tensor by shape or index
        // Let's assume index 0 is landmarks, index 1 is score.
        // But in `blazelandmark.py`:
        // out_landmark_idx = output_details[0]['index']
        // out_flag_idx = output_details[1]['index']
        // So it matches.

        let (_shape_lm, landmarks_slice) = outputs[0].try_extract_tensor::<f32>()?;
        let (_shape_score, score_slice) = outputs[1].try_extract_tensor::<f32>()?; // flag

        let score_val = score_slice[0];

        // 5. Denormalize
        // Landmarks are typically normalized 0..resolution
        // We need to convert them to ROI pixels 0..resolution
        // Wait, Python code: `out2 = out2/self.resolution` -> normalized 0..1
        // Then `denormalize_landmarks` multiplies by resolution.
        // My `denormalize_landmarks` expects 0..resolution.
        
        // So I take raw output (which is directly 0..resolution usually? No check pyt)
        // Python: `out2 = out2 / self.resolution`
        // So the raw output IS 0..resolution (e.g. 0..192).
        
        let mut landmarks_roi_2d = Vec::new();
        let mut z_coords = Vec::new();
        
        for i in 0..self.num_landmarks {
            let offset = i * self.num_dims;
            let lx = landmarks_slice[offset];
            let ly = landmarks_slice[offset+1];
            let lz = if self.num_dims > 2 { landmarks_slice[offset+2] } else { 0.0 };
            
            landmarks_roi_2d.push((lx, ly));
            z_coords.push(lz);
        }

        let landmarks_orig_2d = denormalize_landmarks(
            &landmarks_roi_2d, 
            xc, yc, theta, scale, 
            (self.input_size.0 as u32, self.input_size.1 as u32)
        );

        // Combine with Z
        // Z also needs scaling? Python: `landmark[:,:2] *= self.resolution`
        // Z is relative to scale.
        // `landmark = (affine[:,:2] @ landmark[:,:2].T + affine[:,2:]).T`
        
        let mut final_landmarks = Vec::new();
        for (i, (ox, oy)) in landmarks_orig_2d.iter().enumerate() {
            // Z needs to be scaled by (scale / resolution) theoretically?
            // "z coordinate is relative to the bounding box size"
            
            let scale_factor = scale / self.input_size.0 as f32; 
            let oz = z_coords[i] * scale_factor;
            
            final_landmarks.push((*ox, *oy, oz));
        }

        Ok((final_landmarks, score_val))
    }
}
