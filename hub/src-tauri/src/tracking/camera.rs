use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;
use anyhow::{Result, anyhow};
use image::{ImageBuffer, Rgb};
use crate::tracking::types::CameraInfo;

pub struct CameraManager {
    camera: Option<Camera>,
}

impl CameraManager {
    pub fn new() -> Self {
        Self {
            camera: None,
        }
    }

    pub fn get_cameras(&self) -> Vec<CameraInfo> {
        let cameras = match nokhwa::query(nokhwa::utils::ApiBackend::Auto) {
            Ok(cams) => cams,
            Err(_) => return vec![],
        };

        cameras.into_iter().map(|info| {
            let index = info.index().as_index().unwrap() as i32;
            CameraInfo {
                index,
                name: info.human_name(),
                backend: Some(info.description().to_string()),
                resolution: None,
                fps: None,
            }
        }).collect()
    }

    pub fn start(&mut self, index: usize) -> Result<()> {
        let index = CameraIndex::Index(index as u32);
        let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        
        let mut camera = Camera::new(index, requested)?;
        camera.open_stream()?;
        self.camera = Some(camera);
        Ok(())
    }

    pub fn get_frame(&mut self) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>> {
        if let Some(ref mut camera) = self.camera {
            let frame = camera.frame()?;
            let buffer = frame.decode_image::<RgbFormat>()?;
            Ok(buffer)
        } else {
            Err(anyhow!("Camera not started"))
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut camera) = self.camera.take() {
            let _ = camera.stop_stream();
        }
    }
}
