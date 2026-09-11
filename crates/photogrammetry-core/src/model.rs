use crate::{Result, camera::Camera, math::V3};
#[derive(Clone)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
    pub focal: f64,
}
impl Image {
    pub fn validate(&self) -> Result<()> {
        if self.width < 48
            || self.height < 48
            || self.width > 2048
            || self.height > 2048
            || self.rgb.len() != self.width * self.height * 3
        {
            return Err(crate::error(
                "Expected RGB images between 48 and 2048 pixels per side",
            ));
        }
        if !self.focal.is_finite() || self.focal < 20. || self.focal > 20000. {
            return Err(crate::error("Invalid focal length in pixels"));
        }
        Ok(())
    }
    pub fn gray(&self) -> Vec<f32> {
        self.rgb
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) / 255.)
            .collect()
    }
    pub fn camera(&self) -> Camera {
        Camera::identity(self.focal, self.width as f64 / 2., self.height as f64 / 2.)
    }
}
#[derive(Clone, Debug)]
pub struct Point {
    pub position: V3,
    pub color: [u8; 3],
    pub observations: Vec<(usize, usize)>,
}
#[derive(Clone, Debug)]
pub struct Reconstruction {
    pub cameras: Vec<Option<Camera>>,
    pub points: Vec<Point>,
    pub input_images: usize,
    pub reprojection_rmse: f64,
}
