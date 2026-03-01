use crate::math::{PerspectiveProjection, Projection};

#[derive(Clone, Debug)]
pub struct Camera {
    /// Projection matrix for camera
    pub projection: Projection,
    /// Current physical size of the camera render target
    pub physical_size: [u32; 2],
}

impl Camera {
    pub fn update(&mut self, width: u32, height: u32) {
        self.projection.update(width as f32, height as f32);
        self.physical_size = [width, height];
    }

    pub fn perspective(fov: f32, near: f32, far: f32) -> Camera {
        let mut proj = PerspectiveProjection::default();
        proj.fov = fov;
        proj.near = near;
        proj.far = far;

        Camera {
            projection: proj.into(),
            physical_size: [16, 16],
        }
    }
}
