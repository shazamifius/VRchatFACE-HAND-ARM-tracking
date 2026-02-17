#[cfg(test)]
mod tests {
    use crate::tracking::blaze::config::{generate_anchors, get_face_short_range_config};
    use crate::tracking::blaze::utils::{resize_pad, Detection, weighted_non_max_suppression};
    use image::{DynamicImage, Rgba, ImageBuffer};

    #[test]
    fn test_face_anchors_generation() {
        let (options, config) = get_face_short_range_config();
        let anchors = generate_anchors(&options);
        
        println!("Generated {} anchors", anchors.len());
        assert_eq!(anchors.len(), config.num_anchors, "Number of anchors should match config");
        assert_eq!(anchors.len(), 896, "Face Short Range should have 896 anchors");
        
        // Check first anchor (should be normalized)
        let a0 = &anchors[0];
        assert!(a0.x_center >= 0.0 && a0.x_center <= 1.0);
        assert!(a0.y_center >= 0.0 && a0.y_center <= 1.0);
    }

    #[test]
    fn test_resize_pad() {
        // Create 200x100 image
        let img = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(200, 100, Rgba([255, 0, 0, 255])));
        
        // Target 128x128
        let (resized, scale, (pad_w, pad_h)) = resize_pad(&img, 128, 128);
        
        assert_eq!(resized.width(), 128);
        assert_eq!(resized.height(), 128);
        
        // Aspect ratio of source is 2:1
        // Target is 1:1
        // Should fit width (128) and scale height to 64
        // Logic: 
        // w/h = 2/1. target w/h = 1.
        // source wider -> fit width.
        // scale = 128 / 200 = 0.64
        // new_h = 100 * 0.64 = 64.
        // pad_h = (128 - 64) / 2 = 32.
        
        println!("Scale: {}, Pad: {}x{}", scale, pad_w, pad_h);
        
        assert_eq!(scale, 128.0 / 200.0);
        assert_eq!(pad_w, 0.0);
        assert_eq!(pad_h, 32.0);
    }

    #[test]
    fn test_nms() {
        let mut detections = vec![
            Detection {
                score: 0.9,
                class_id: 0,
                ymin: 10.0, xmin: 10.0, ymax: 20.0, xmax: 20.0,
                keypoints: vec![]
            },
            Detection { // Overlapping with first (almost same)
                score: 0.8,
                class_id: 0,
                ymin: 10.5, xmin: 10.5, ymax: 20.5, xmax: 20.5,
                keypoints: vec![]
            },
            Detection { // Distinct
                score: 0.95,
                class_id: 0,
                ymin: 100.0, xmin: 100.0, ymax: 120.0, xmax: 120.0,
                keypoints: vec![]
            }
        ];

        let filtered = weighted_non_max_suppression(&mut detections, 0.3);
        
        assert_eq!(filtered.len(), 2, "Should merge overlapping boxes");
        // The one with 0.95 and the merged one (0.9 and 0.8)
    }
}
