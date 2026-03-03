use anyhow::Result;
use ort::session::{Session, builder::GraphOptimizationLevel};
use image::{DynamicImage, GenericImageView};
use ndarray::Array4;
use crate::tracking::blaze::utils::{extract_roi, denormalize_landmarks};

pub struct BlazeLandmark {
    session: Session,
    input_size: (usize, usize), 
    num_dims: usize, // 2 or 3 (x, y, [z])
    is_nchw: bool,
}

impl BlazeLandmark {
    pub fn new(model_path: &str, input_size: usize, _num_landmarks: usize, num_dims: usize) -> Result<Self> {
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)?;

        let mut is_nchw = true;
        if let Some(input) = session.inputs().get(0) {
            let name = input.name();
            println!("[Rust] Landmark input name: {}", name);
            if name.contains("_1") || name.contains(".1") || name.to_lowercase().contains("nhwc") {
                is_nchw = false;
                println!("[Rust] Model '{}' detected as NHWC.", name);
            } else {
                println!("[Rust] Model '{}' detected as NCHW.", name);
            }
        }

        Ok(Self {
            session,
            input_size: (input_size, input_size),
            num_dims,
            is_nchw,
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
    ) -> Result<(Vec<(f32, f32, f32)>, f32, Option<f32>)> { // (Keypoints, Score, Handedness?)
        
        // 1. Extract ROI
        let roi_img = extract_roi(image, xc, yc, theta, scale, (self.input_size.0 as u32, self.input_size.1 as u32));

        // 2. Preprocess
        // Normalize 0..1 or -1..1?
        // BlazeLandmark usually expects 0..1
        // Let's check blazelandmark.py -> preprocess -> return x / 255.0
        // blazedetector was -1..1
        
        // Create tensor based on layout
        let mut input_tensor = if self.is_nchw {
            Array4::<f32>::zeros((1, 3, self.input_size.1, self.input_size.0))
        } else {
            Array4::<f32>::zeros((1, self.input_size.1, self.input_size.0, 3))
        };

        for (x, y, pixel) in roi_img.pixels() {
            let r = pixel[0] as f32 / 255.0;
            let g = pixel[1] as f32 / 255.0;
            let b = pixel[2] as f32 / 255.0;
            
            if self.is_nchw {
                input_tensor[[0, 0, y as usize, x as usize]] = r;
                input_tensor[[0, 1, y as usize, x as usize]] = g;
                input_tensor[[0, 2, y as usize, x as usize]] = b;
            } else {
                input_tensor[[0, y as usize, x as usize, 0]] = r;
                input_tensor[[0, y as usize, x as usize, 1]] = g;
                input_tensor[[0, y as usize, x as usize, 2]] = b;
            }
        }

        // 3. Inference
        let input_value = ort::value::Value::from_array(input_tensor)?;
        let input_name = self.session.inputs()[0].name().to_string();
        let inputs = ort::inputs![input_name => input_value];
        let outputs = self.session.run(inputs)?;

        // 4. Extract
        // Output indices:
        // Face Mesh (2 outputs): landmarks, score
        // Hand (3 outputs): landmarks, score, handedness
        
        // We try to find tensor by shape or index
        // Let's assume index 0 is landmarks, index 1 is score.
        // But in `blazelandmark.py`:
        // out_landmark_idx = output_details[0]['index']
        // out_flag_idx = output_details[1]['index']
        // So it matches.

        let (shape_lm, landmarks_slice) = outputs[0].try_extract_tensor::<f32>()?;
        let (_shape_score, score_slice) = outputs[1].try_extract_tensor::<f32>()?; // flag
        let score_val = score_slice[0];

        if true {
             println!("[AI] Landmark Output Shape: {:?} | Slice Len: {}", shape_lm, landmarks_slice.len());
        }

        let mut handedness_val = None;
        if outputs.len() >= 3 {
             if let Ok((_shape_h, h_slice)) = outputs[2].try_extract_tensor::<f32>() {
                 if !h_slice.is_empty() {
                    handedness_val = Some(h_slice[0]);
                 }
             }
        }

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
        
        let num_pts = landmarks_slice.len() / self.num_dims;
        if num_pts == 0 { return Err(anyhow::anyhow!("No landmarks in output tensor")); }

        for i in 0..num_pts {
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
        // Z is relative to scale.
        
        let mut final_landmarks = Vec::new();
        // Calculate the scale factor for depth. Z coordinate is relative to the bounding box scale 
        // across the original image, mapped from the model's pixel space.
        let scale_factor = scale / self.input_size.0 as f32; 
        
        for (i, (ox, oy)) in landmarks_orig_2d.iter().enumerate() {
            let oz = z_coords[i] * scale_factor;
            final_landmarks.push((*ox, *oy, oz));
        }

        Ok((final_landmarks, score_val, handedness_val))
    }
}
