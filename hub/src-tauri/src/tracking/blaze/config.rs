use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorOptions {
    pub num_layers: usize,
    pub min_scale: f32,
    pub max_scale: f32,
    pub input_size_height: usize,
    pub input_size_width: usize,
    pub anchor_offset_x: f32,
    pub anchor_offset_y: f32,
    pub strides: Vec<usize>,
    pub aspect_ratios: Vec<f32>,
    pub reduce_boxes_in_lowest_layer: bool,
    pub interpolated_scale_aspect_ratio: f32,
    pub fixed_anchor_size: bool,
}

#[derive(Debug, Clone)]
pub struct BlazeConfig {
    pub num_classes: usize,
    pub num_anchors: usize,
    pub num_coords: usize,
    pub score_clipping_thresh: f32,
    pub x_scale: f32,
    pub y_scale: f32,
    pub h_scale: f32,
    pub w_scale: f32,
    pub min_score_thresh: f32,
    pub min_suppression_threshold: f32,
    pub num_keypoints: usize,
    pub detection2roi_method: String,
    pub kp1: usize,
    pub kp2: usize,
    pub theta0: f32,
    pub dscale: f32,
    pub dy: f32,
}

#[derive(Debug, Clone)]
pub struct Anchor {
    pub x_center: f32,
    pub y_center: f32,
    pub w: f32,
    pub h: f32,
}

/// Calculate scale based on stride index
fn calculate_scale(min_scale: f32, max_scale: f32, stride_index: usize, num_strides: usize) -> f32 {
    if num_strides == 1 {
        (max_scale + min_scale) * 0.5
    } else {
        min_scale + (max_scale - min_scale) * stride_index as f32 / (num_strides as f32 - 1.0)
    }
}

/// Generate anchors based on options (Port from SSDAnchorsCalculator)
pub fn generate_anchors(options: &AnchorOptions) -> Vec<Anchor> {
    let mut anchors = Vec::new();
    let num_layers = options.num_layers;
    let strides_size = options.strides.len();
    assert_eq!(num_layers, strides_size);

    let mut layer_id = 0;
    while layer_id < strides_size {
        let mut anchor_height = Vec::new();
        let mut anchor_width = Vec::new();
        let mut aspect_ratios = Vec::new();
        let mut scales = Vec::new();

        let mut last_same_stride_layer = layer_id;
        while last_same_stride_layer < strides_size && options.strides[last_same_stride_layer] == options.strides[layer_id] {
            let scale = calculate_scale(options.min_scale, options.max_scale, last_same_stride_layer, strides_size);

            if last_same_stride_layer == 0 && options.reduce_boxes_in_lowest_layer {
                aspect_ratios.push(1.0);
                aspect_ratios.push(2.0);
                aspect_ratios.push(0.5);
                scales.push(0.1);
                scales.push(scale);
                scales.push(scale);
            } else {
                for &ratio in &options.aspect_ratios {
                    aspect_ratios.push(ratio);
                    scales.push(scale);
                }

                if options.interpolated_scale_aspect_ratio > 0.0 {
                    let scale_next = if last_same_stride_layer == strides_size - 1 {
                        1.0
                    } else {
                        calculate_scale(options.min_scale, options.max_scale, last_same_stride_layer + 1, strides_size)
                    };
                    scales.push((scale * scale_next).sqrt());
                    aspect_ratios.push(options.interpolated_scale_aspect_ratio);
                }
            }
            last_same_stride_layer += 1;
        }

        for i in 0..aspect_ratios.len() {
            let ratio_sqrt = aspect_ratios[i].sqrt();
            anchor_height.push(scales[i] / ratio_sqrt);
            anchor_width.push(scales[i] * ratio_sqrt);
        }

        let stride = options.strides[layer_id];
        let feature_map_height = (options.input_size_height as f32 / stride as f32).ceil() as usize;
        let feature_map_width = (options.input_size_width as f32 / stride as f32).ceil() as usize;

        for y in 0..feature_map_height {
            for x in 0..feature_map_width {
                for anchor_id in 0..anchor_height.len() {
                    let x_center = (x as f32 + options.anchor_offset_x) / feature_map_width as f32;
                    let y_center = (y as f32 + options.anchor_offset_y) / feature_map_height as f32;

                    let (w, h) = if options.fixed_anchor_size {
                        (1.0, 1.0)
                    } else {
                        (anchor_width[anchor_id], anchor_height[anchor_id])
                    };

                    anchors.push(Anchor {
                        x_center,
                        y_center,
                        w,
                        h,
                    });
                }
            }
        }

        layer_id = last_same_stride_layer;
    }

    anchors
}

// --- Predefined Configs ---

// Face Short Range (Front Camera)
// Removed static const because Vec cannot be const. Use the getter function.\


pub fn get_face_short_range_config() -> (AnchorOptions, BlazeConfig) {
    let anchor_options = AnchorOptions {
        num_layers: 4,
        min_scale: 0.1484375,
        max_scale: 0.75,
        input_size_height: 128,
        input_size_width: 128,
        anchor_offset_x: 0.5,
        anchor_offset_y: 0.5,
        strides: vec![8, 16, 16, 16],
        aspect_ratios: vec![1.0],
        reduce_boxes_in_lowest_layer: false,
        interpolated_scale_aspect_ratio: 1.0,
        fixed_anchor_size: true,
    };

    let model_config = BlazeConfig {
        num_classes: 1,
        num_anchors: 896,
        num_coords: 16,
        score_clipping_thresh: 100.0,
        x_scale: 128.0,
        y_scale: 128.0,
        h_scale: 128.0,
        w_scale: 128.0,
        min_score_thresh: 0.5,
        min_suppression_threshold: 0.3,
        num_keypoints: 6,
        detection2roi_method: "box".to_string(), // Face uses 'box'
        kp1: 1, // Left Eye
        kp2: 0, // Right Eye
        theta0: 0.0,
        dscale: 1.5,
        dy: 0.0,
    };

    (anchor_options, model_config)
}


// Palm Detection (Lite/Full are similar geometry, usually 192x192)
// MediaPipe Hands uses 128x128 for Lite, 192x192 for Full.
// The downloaded model is `palm_detection_mediapipe_2023feb.onnx`.
// Opencv zoo docs say input is 192x192.
pub fn get_palm_detection_config() -> (AnchorOptions, BlazeConfig) {
    let _anchor_options = AnchorOptions {
        num_layers: 4,
        min_scale: 0.1484375,
        max_scale: 0.75,
        input_size_height: 192,
        input_size_width: 192,
        anchor_offset_x: 0.5,
        anchor_offset_y: 0.5,
        strides: vec![8, 16, 16, 16], // Verify strides for 192 input
        aspect_ratios: vec![1.0],
        reduce_boxes_in_lowest_layer: false,
        interpolated_scale_aspect_ratio: 1.0,
        fixed_anchor_size: true,
    };
    // Note: Standard MP Palm might have different strides: 4, 8, 16, 32?
    // Let's assume standard SSD logic.
    // If input 192: 
    // Stride 8 -> 24x24
    // Stride 16 -> 12x12
    // Stride 16 -> 12x12 (Wait, usually increasing?)
    // Stride 32 -> 6x6
    // Actually, let's use the standard values found in other implementations for 192x192.
    // Stride: 4, 8, 16, 32.
    // num_anchors = 2016.
    
    let anchor_options_corrected = AnchorOptions {
        num_layers: 4,
        min_scale: 0.1484375,
        max_scale: 0.75,
        input_size_height: 192,
        input_size_width: 192,
        anchor_offset_x: 0.5,
        anchor_offset_y: 0.5,
        strides: vec![8, 16, 16, 16], // [8, 16, 16, 16] yields 2016 anchors with 192x192 input
        aspect_ratios: vec![1.0], 
        reduce_boxes_in_lowest_layer: false, 
        interpolated_scale_aspect_ratio: 1.0,
        fixed_anchor_size: true,
    };

    let model_config = BlazeConfig {
        num_classes: 1,
        num_anchors: 2016, // (48*48 + 24*24 + 12*12 + 6*6) * 2? No.
        // 192/4 = 48. 48*48 = 2304.
        // It depends on aspect ratios per layer.
        // Standard Palm: 2 anchors per point.
        // 48*48*2 + 24*24*6 + ...
        // Actually, let's trust the `num_anchors` count from standard TFLite graph if possible.
        // For now, use 2016 which is common for 192x192.
        num_coords: 18, // 4 box + 7 keypoints * 2
        score_clipping_thresh: 100.0,
        x_scale: 192.0,
        y_scale: 192.0,
        h_scale: 192.0,
        w_scale: 192.0,
        min_score_thresh: 0.5,
        min_suppression_threshold: 0.3,
        num_keypoints: 7, 
        detection2roi_method: "box".to_string(), // Palm uses 'box' (from wrist/middle finger)
        kp1: 0, // Wrist
        kp2: 2, // Middle Finger Base
        theta0: 90.0 * std::f32::consts::PI / 180.0, // Hand is usually vertical?
        dscale: 2.6, // Box enlargement
        dy: -0.1, // Shift up/down
    };

    (anchor_options_corrected, model_config)
}

pub fn get_hand_landmark_config() -> (u32, u32) {
    (224, 224) // Input size
}
