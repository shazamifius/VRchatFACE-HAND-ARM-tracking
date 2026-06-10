use anyhow::Result;
mod blaze;
use blaze::detector::BlazeDetector;
use blaze::config::{get_face_short_range_config};

fn main() -> Result<()> {
    let models_dir = "models";
    let face_path = format!("{}/face_detection_short_range.onnx", models_dir);
    
    println!("Testing Model Loading: {}", face_path);
    
    let (anchor_options, config) = get_face_short_range_config();
    let detector = BlazeDetector::new(&face_path, config, anchor_options);
    
    match detector {
        Ok(det) => {
            println!("Success: Model Loaded!");
            // Check session inputs directly if possible or trust the constructor
        },
        Err(e) => {
            println!("Error Loading Model: {}", e);
        }
    }
    
    Ok(())
}
