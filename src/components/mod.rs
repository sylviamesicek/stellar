use std::time::Duration;

use crate::math::{PerspectiveProjection, Projection};

mod editor_controller;
mod panorbit_controller;

pub use panorbit_controller::{PanOrbitController, update_pan_orbit_camera};

#[derive(Clone, Debug)]
pub struct Camera {
    /// Projection matrix for camera
    pub projection: Projection,
    /// Current physical size of the camera render target
    physical_size: [u32; 2],
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

    pub fn physical_size(&self) -> [u32; 2] {
        self.physical_size
    }

    pub fn logical_size(&self) -> glam::Vec2 {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub enum Pipeline {
    Mandlebulb,
    Sierpinski,
}

#[derive(Clone, Debug)]
pub struct Global {
    pub time: Duration,
    pub pre_saturation: f32,
    pub post_saturation: f32,
    pub gamma: f32,
    pub exposure: f32,

    pub pipeline: Pipeline,
}

impl Global {
    pub const DEFAULT: Self = Self {
        time: Duration::ZERO,
        pre_saturation: 1.0,
        post_saturation: 1.0,
        gamma: 1.0,
        exposure: 0.0,
        pipeline: Pipeline::Mandlebulb,
    };

    pub fn new() -> Self {
        Self::DEFAULT
    }
}

impl Default for Global {
    fn default() -> Self {
        Self::DEFAULT
    }
}
