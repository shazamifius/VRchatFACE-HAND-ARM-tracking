use image::{DynamicImage, GenericImageView, Rgba, ImageBuffer};
use nalgebra as na;
use crate::tracking::blaze::config::{Anchor, BlazeConfig};

/// Resize and pad image to target size while maintaining aspect ratio
pub fn resize_pad(img: &DynamicImage, target_w: u32, target_h: u32) -> (DynamicImage, f32, (f32, f32)) {
    let (w, h) = img.dimensions();
    let scale;
    let new_w;
    let new_h;
    let pad_w;
    let pad_h;

    if w as f32 / h as f32 > target_w as f32 / target_h as f32 {
        scale = target_w as f32 / w as f32;
        new_w = target_w;
        new_h = (h as f32 * scale) as u32;
        pad_w = 0;
        pad_h = (target_h - new_h) / 2;
    } else {
        scale = target_h as f32 / h as f32;
        new_w = (w as f32 * scale) as u32;
        new_h = target_h;
        pad_w = (target_w - new_w) / 2;
        pad_h = 0;
    }

    let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Triangle);
    let mut padded = ImageBuffer::from_pixel(target_w, target_h, Rgba([0, 0, 0, 255]));

    image::imageops::overlay(&mut padded, &resized, pad_w as i64, pad_h as i64);

    (DynamicImage::ImageRgba8(padded), scale, (pad_w as f32, pad_h as f32))
}

#[derive(Debug, Clone)]
pub struct Detection {
    pub score: f32,
    pub class_id: usize,
    pub ymin: f32,
    pub xmin: f32,
    pub ymax: f32,
    pub xmax: f32,
    pub keypoints: Vec<(f32, f32)>, // (x, y)
}

/// Decode raw boxes from model output using anchors
pub fn decode_boxes(
    raw_boxes: &[f32], 
    raw_scores: &[f32], 
    anchors: &[Anchor], 
    config: &BlazeConfig
) -> Vec<Detection> {
    let mut detections = Vec::new();

    // raw_boxes shape: [num_anchors, num_coords]
    // raw_scores shape: [num_anchors, num_classes] (often 1)

    for (i, anchor) in anchors.iter().enumerate() {
        let score_offset = i * config.num_classes;
        if score_offset >= raw_scores.len() { break; }
        
        // Find max score for this anchor
        let score_raw = raw_scores[score_offset];
        // Sigmoid
        let score_val = 1.0 / (1.0 + (-score_raw.clamp(-100.0, 100.0)).exp()); 
        
        if score_val < config.min_score_thresh {
            continue;
        }

        let box_offset = i * config.num_coords;
        if box_offset + 3 >= raw_boxes.len() { break; }
        
        let cy = raw_boxes[box_offset] / config.y_scale * anchor.h + anchor.y_center;
        let cx = raw_boxes[box_offset + 1] / config.x_scale * anchor.w + anchor.x_center;
        let h = raw_boxes[box_offset + 2] / config.h_scale * anchor.h;
        let w = raw_boxes[box_offset + 3] / config.w_scale * anchor.w;

        let ymin = cy - h / 2.0;
        let xmin = cx - w / 2.0;
        let ymax = cy + h / 2.0;
        let xmax = cx + w / 2.0;

        let mut keypoints = Vec::new();
        for k in 0..config.num_keypoints {
            let offset = box_offset + 4 + k * 2;
            let kx = raw_boxes[offset] / config.x_scale * anchor.w + anchor.x_center;
            let ky = raw_boxes[offset + 1] / config.y_scale * anchor.h + anchor.y_center;
            keypoints.push((kx, ky));
        }

        detections.push(Detection {
            score: score_val,
            class_id: 0, 
            ymin,
            xmin,
            ymax,
            xmax,
            keypoints,
        });
    }

    detections
}

/// Calculate ROI from detection for landmark model
pub fn detection2roi(
    detection: &Detection, 
    config: &BlazeConfig
) -> (f32, f32, f32, f32) {
    // Returns (xc, yc, scale, theta)

    let xc;
    let yc;
    let mut scale;
    let theta;

    if config.detection2roi_method == "box" {
        // Face detection uses box center
        xc = (detection.xmin + detection.xmax) / 2.0;
        yc = (detection.ymin + detection.ymax) / 2.0;
        scale = (detection.xmax - detection.xmin).abs(); // width
        theta = 0.0;
    } else if config.detection2roi_method == "alignment" {
        // Hand/Pose uses keypoints alignment
        let kp1 = detection.keypoints[config.kp1];
        let kp2 = detection.keypoints[config.kp2];
        
        xc = kp1.0;
        yc = kp1.1;
        
        let x1 = kp2.0;
        let y1 = kp2.1;

        scale = ((xc - x1).powi(2) + (yc - y1).powi(2)).sqrt() * 2.0;
        theta = (yc - y1).atan2(xc - x1) - config.theta0;
    } else {
        // Fallback
        xc = 0.5;
        yc = 0.5;
        scale = 1.0;
        theta = 0.0;
    }

    // Apply shifts and scale
    let yc = yc + config.dy * scale;
    scale = scale * config.dscale;

    (xc, yc, scale, theta)
}

/// Extract ROI from image using affine transform (Crop + Rotate + Resize)
pub fn extract_roi(
    image: &DynamicImage,
    xc: f32,
    yc: f32, 
    theta: f32,
    scale: f32,
    target_size: (u32, u32)
) -> DynamicImage {
    let (w, h) = target_size;
    let mut output = ImageBuffer::from_pixel(w, h, Rgba([0, 0, 0, 255]));

    let img_w = image.width() as f32;
    let img_h = image.height() as f32;

    // Calculate affine matrix
    // specific to BlazeFace/Hand logic:
    // the "scale" is the size of the box
    // we want to map the box (xc, yc, scale, theta) to the output (0, 0, w, h)
    
    // Translation to center
    let t1 = na::Matrix3::new(
        1.0, 0.0, -xc,
        0.0, 1.0, -yc,
        0.0, 0.0, 1.0
    );

    // Rotation
    let c = theta.cos();
    let s = theta.sin();
    let r = na::Matrix3::new(
         c,   s, 0.0,
        -s,   c, 0.0,
         0.0, 0.0, 1.0
    );

    // Scale
    // We want 'scale' pixels from source to map to 'w' pixels in target
    let s_factor = w as f32 / scale;
    let s_mat = na::Matrix3::new(
        s_factor, 0.0, 0.0,
        0.0, s_factor, 0.0,
        0.0, 0.0, 1.0
    );

    // Example transform: Target <- Scale * Rotation * Translation * Source
    // Actually we iterate over Target pixels and want Source pixels (Inverse mapping)
    // Source = Translation^-1 * Rotation^-1 * Scale^-1 * Target_Center_Offset * Target
    
    // Let's stick to forward logic conceptually first. 
    // We want to sample source pixel (sx, sy) for target pixel (tx, ty).
    
    // Matrix M maps Source -> Target
    // We need Inv(M) to map Target -> Source
    
    // M = S * R * T
    // T shifts (xc, yc) to (0, 0)
    // R rotates
    // S scales to target size BUT we also need to shift back to target center (w/2, h/2)
    
    let t2 = na::Matrix3::new(
        1.0, 0.0, w as f32 / 2.0,
        0.0, 1.0, h as f32 / 2.0,
        0.0, 0.0, 1.0
    );
    
    let m = t2 * s_mat * r * t1;
    let m_inv = m.try_inverse().unwrap_or(na::Matrix3::identity());

    // Iterate
    for ty in 0..h {
        for tx in 0..w {
            let target_point = na::Vector3::new(tx as f32, ty as f32, 1.0);
            let source_point = m_inv * target_point;

            let sx = source_point.x;
            let sy = source_point.y;

            if sx >= 0.0 && sx < img_w && sy >= 0.0 && sy < img_h {
                let pixel = sample_bilinear(image, sx, sy);
                output.put_pixel(tx, ty, pixel);
            }
        }
    }

    DynamicImage::ImageRgba8(output)
}

fn sample_bilinear(image: &DynamicImage, x: f32, y: f32) -> Rgba<u8> {
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(image.width() - 1);
    let y1 = (y0 + 1).min(image.height() - 1);

    let dx = x - x0 as f32;
    let dy = y - y0 as f32;

    let p00 = image.get_pixel(x0, y0);
    let p10 = image.get_pixel(x1, y0);
    let p01 = image.get_pixel(x0, y1);
    let p11 = image.get_pixel(x1, y1);

    let mut final_pixel = Rgba([0, 0, 0, 255]);

    for c in 0..3 {
        let val = (1.0 - dx) * (1.0 - dy) * p00[c] as f32
                + dx * (1.0 - dy) * p10[c] as f32
                + (1.0 - dx) * dy * p01[c] as f32
                + dx * dy * p11[c] as f32;
        final_pixel[c] = val as u8;
    }

    final_pixel
}

/// Weighted Non-Max Suppression
pub fn weighted_non_max_suppression(
    detections: &mut Vec<Detection>, 
    min_suppression_threshold: f32
) -> Vec<Detection> {
    // Sort by score descending
    detections.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let mut output_detections = Vec::new();
    let mut remaining = detections.clone();

    while !remaining.is_empty() {
        let detection = remaining[0].clone();
        
        // Find overlapping detections
        let mut overlapping = Vec::new();
        let mut non_overlapping = Vec::new();

        for other in remaining.iter() {
            let iou = intersection_over_union(&detection, other);
            if iou > min_suppression_threshold {
                overlapping.push(other.clone());
            } else {
                non_overlapping.push(other.clone());
            }
        }

        // Weighted average
        if !overlapping.is_empty() {
            let mut weighted = detection.clone();
            let mut total_score = 0.0;
            let mut w_ymin = 0.0;
            let mut w_xmin = 0.0;
            let mut w_ymax = 0.0;
            let mut w_xmax = 0.0;
            // Keypoints TODO if needed

            for item in &overlapping {
                total_score += item.score;
                w_ymin += item.ymin * item.score;
                w_xmin += item.xmin * item.score;
                w_ymax += item.ymax * item.score;
                w_xmax += item.xmax * item.score;
            }

            if total_score > 0.0 {
                weighted.ymin = w_ymin / total_score;
                weighted.xmin = w_xmin / total_score;
                weighted.ymax = w_ymax / total_score;
                weighted.xmax = w_xmax / total_score;
            }
            output_detections.push(weighted);
        }

        remaining = non_overlapping;
    }

    output_detections
}

fn intersection_over_union(a: &Detection, b: &Detection) -> f32 {
    let x_left = a.xmin.max(b.xmin);
    let y_top = a.ymin.max(b.ymin);
    let x_right = a.xmax.min(b.xmax);
    let y_bottom = a.ymax.min(b.ymax);

    if x_right < x_left || y_bottom < y_top {
        return 0.0;
    }

    let intersection_area = (x_right - x_left) * (y_bottom - y_top);
    let area_a = (a.xmax - a.xmin) * (a.ymax - a.ymin);
    let area_b = (b.xmax - b.xmin) * (b.ymax - b.ymin);


    intersection_area / (area_a + area_b - intersection_area)
}

/// Convert landmarks from ROI coordinates back to original image coordinates
pub fn denormalize_landmarks(
    landmarks_roi: &[(f32, f32)], // (x, y) in 0..resolution
    xc: f32,
    yc: f32,
    theta: f32,
    scale: f32,
    resolution: (u32, u32)
) -> Vec<(f32, f32)> {
    let (w, h) = resolution;

    // We need the transformation Matrix M that maps Source -> Target
    // P_target = M * P_source
    // P_source = M^-1 * P_target
    
    // Construct M (Same as in extract_roi)
    let t1 = na::Matrix3::new(
        1.0, 0.0, -xc,
        0.0, 1.0, -yc,
        0.0, 0.0, 1.0
    );

    let c = theta.cos();
    let s = theta.sin();
    let r = na::Matrix3::new(
         c,   s, 0.0,
        -s,   c, 0.0,
         0.0, 0.0, 1.0
    );

    let s_factor = w as f32 / scale;
    let s_mat = na::Matrix3::new(
        s_factor, 0.0, 0.0,
        0.0, s_factor, 0.0,
        0.0, 0.0, 1.0
    );
     
    let t2 = na::Matrix3::new(
        1.0, 0.0, w as f32 / 2.0,
        0.0, 1.0, h as f32 / 2.0,
        0.0, 0.0, 1.0
    );
    
    let m = t2 * s_mat * r * t1;
    let m_inv = m.try_inverse().unwrap_or(na::Matrix3::identity());

    let mut landmarks_orig = Vec::new();

    for (lx, ly) in landmarks_roi {
        // Points are already in 0..resolution (pixels in ROI)
        let target_point = na::Vector3::new(*lx, *ly, 1.0);
        let source_point = m_inv * target_point;
        landmarks_orig.push((source_point.x, source_point.y));
    }

    landmarks_orig
}

