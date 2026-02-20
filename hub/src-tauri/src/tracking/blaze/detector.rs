use anyhow::Result;
use ort::session::{Session, builder::GraphOptimizationLevel};
use image::{DynamicImage, GenericImageView};
use ndarray::Array4;
use crate::tracking::blaze::config::{BlazeConfig, Anchor, generate_anchors, AnchorOptions};
use crate::tracking::blaze::utils::{resize_pad, decode_boxes, weighted_non_max_suppression, Detection};

pub struct BlazeDetector {
    session: Session,
    config: BlazeConfig,
    anchors: Vec<Anchor>,
    input_size: (usize, usize), // (w, h)
}

impl BlazeDetector {
    /// Load model and initialize anchors
    pub fn new(model_path: &str, config: BlazeConfig, anchor_options: AnchorOptions) -> Result<Self> {
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)?;

        let anchors = generate_anchors(&anchor_options);
        
        Ok(Self {
            session,
            config,
            anchors,
            input_size: (anchor_options.input_size_width, anchor_options.input_size_height),
        })
    }

    /// Run inference on an image
    pub fn detect(&mut self, image: &DynamicImage) -> Result<Vec<Detection>> {
        // 1. Preprocess
        let (resized_img, scale, (pad_w, pad_h)) = resize_pad(image, self.input_size.0 as u32, self.input_size.1 as u32);
        
        // Convert to Tensor [1, H, W, 3] (Float32, normalized 0..1)
        let mut input_tensor = Array4::<f32>::zeros((1, self.input_size.1, self.input_size.0, 3));
        for (x, y, pixel) in resized_img.pixels() {
            let r = pixel[0] as f32 / 255.0;
            let g = pixel[1] as f32 / 255.0;
            let b = pixel[2] as f32 / 255.0;
            input_tensor[[0, y as usize, x as usize, 0]] = r;
            input_tensor[[0, y as usize, x as usize, 1]] = g;
            input_tensor[[0, y as usize, x as usize, 2]] = b;
        }

        // 2. Inference
        // Create ORT value from array (takes ownership or view, here we give ownership of the array)
        let input_value = ort::value::Value::from_array(input_tensor)?;
        let input_name = self.session.inputs()[0].name().to_string();
        let inputs = ort::inputs![input_name => input_value]; 
        let outputs = self.session.run(inputs)?;

        // 3. Extract Outputs
        // Usually output[0] = regressors, output[1] = classifiers
        // Check tensor shapes to be sure?
        // Let's assume standard MediaPipe export order: Regressors, Classifiers
        
        // Note: ORT output order might depend on the model.
        // For BlazeFace: 
        // - "regressors" [1, 896, 16]
        // - "classifiers" [1, 896, 1]
        
        // We try to fetch by index or name if known. 
        // Let's try explicit names if possible, otherwise indices.
        // Since we don't know exact names, we rely on the fact that regressors have 'num_coords' in last dim.
        
        // Get first output
        // try_extract_tensor returns (Shape, &[T]) in recent ort
        let (shape0, data0) = outputs[0].try_extract_tensor::<f32>()?;
        let (_shape1, data1) = outputs[1].try_extract_tensor::<f32>()?;
        
        // shape is typically [batch, anchors, coords]
        // We check the last dimension
        let (raw_boxes, raw_scores) = if shape0[2] as usize == self.config.num_coords {
            (data0, data1)
        } else {
            (data1, data0)
        };

        // 4. Post-process
        // Slices are already obtained from data0/data1


        let mut detections = decode_boxes(
            raw_boxes, 
            raw_scores, 
            &self.anchors, 
            &self.config
        );

        // 5. NMS
        let filtered = weighted_non_max_suppression(&mut detections, self.config.min_suppression_threshold);

        // 6. Denormalize to original image
        let mut final_detections = Vec::new();
        for det in filtered {
            let mut d = det.clone();
            // Undo padding and scaling
            // x_new = (x_old * input_size - pad) / scale
            
            // Current coordinates are absolute in input_size domain because of decode_boxes using scale/anchor size
            // Wait, decode_boxes uses config.x_scale which is input_size. So coords are pixels in [0, input_size].
            
            d.xmin = (d.xmin - pad_w) / scale;
            d.xmax = (d.xmax - pad_w) / scale;
            d.ymin = (d.ymin - pad_h) / scale;
            d.ymax = (d.ymax - pad_h) / scale;
            
            for k in 0..d.keypoints.len() {
                d.keypoints[k].0 = (d.keypoints[k].0 - pad_w) / scale;
                d.keypoints[k].1 = (d.keypoints[k].1 - pad_h) / scale;
            }
            
            final_detections.push(d);
        }

        Ok(final_detections)
    }
}
