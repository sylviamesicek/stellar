use std::time::Duration;

use crate::math::{PerspectiveProjection, Projection};

// mod editor_controller;
mod panorbit_controller;

use glam::Vec2;
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

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Pipeline {
    Mandlebulb,
    Sierpinski,
    Standard,
}

#[derive(Clone, Debug)]
pub struct TonemapSettings {
    pub pre_saturation: f32,
    pub post_saturation: f32,
    pub gamma: f32,
    pub exposure: f32,
}

impl Default for TonemapSettings {
    fn default() -> Self {
        Self {
            pre_saturation: 1.0,
            post_saturation: 1.0,
            gamma: 1.0,
            exposure: 0.0,
        }
    }
}

/// Applies a bloom effect to an HDR-enabled 2d or 3d camera.
///
/// Bloom emulates an effect found in real cameras and the human eye,
/// causing halos to appear around very bright parts of the scene.
///
/// See also <https://en.wikipedia.org/wiki/Bloom_(shader_effect)>.
///
/// # Usage Notes
///
/// Often used in conjunction with `bevy_pbr::StandardMaterial::emissive` for 3d meshes.
///
/// Bloom is best used alongside a tonemapping function that desaturates bright colors,
/// such as [`bevy_core_pipeline::tonemapping::Tonemapping::TonyMcMapface`].
///
/// Bevy's implementation uses a parametric curve to blend between a set of
/// blurred (lower frequency) images generated from the camera's view.
/// See <https://starlederer.github.io/bloom/> for a visualization of the parametric curve
/// used in Bevy as well as a visualization of the curve's respective scattering profile.
#[derive(Debug, Clone)]
pub struct BloomSettings {
    /// Controls the baseline of how much the image is scattered (default: 0.15).
    ///
    /// This parameter should be used only to control the strength of the bloom
    /// for the scene as a whole. Increasing it too much will make the scene appear
    /// blurry and over-exposed.
    ///
    /// To make a mesh glow brighter, rather than increase the bloom intensity,
    /// you should increase the mesh's `emissive` value.
    ///
    /// # In energy-conserving mode
    /// The value represents how likely the light is to scatter.
    ///
    /// The value should be between 0.0 and 1.0 where:
    /// * 0.0 means no bloom
    /// * 1.0 means the light is scattered as much as possible
    ///
    /// # In additive mode
    /// The value represents how much scattered light is added to
    /// the image to create the glow effect.
    ///
    /// In this configuration:
    /// * 0.0 means no bloom
    /// * Greater than 0.0 means a proportionate amount of scattered light is added
    pub intensity: f32,

    /// Low frequency contribution boost.
    /// Controls how much more likely the light
    /// is to scatter completely sideways (low frequency image).
    ///
    /// Comparable to a low shelf boost on an equalizer.
    ///
    /// # In energy-conserving mode
    /// The value should be between 0.0 and 1.0 where:
    /// * 0.0 means low frequency light uses base intensity for blend factor calculation
    /// * 1.0 means low frequency light contributes at full power
    ///
    /// # In additive mode
    /// The value represents how much scattered light is added to
    /// the image to create the glow effect.
    ///
    /// In this configuration:
    /// * 0.0 means no bloom
    /// * Greater than 0.0 means a proportionate amount of scattered light is added
    pub low_frequency_boost: f32,

    /// Low frequency contribution boost curve.
    /// Controls the curvature of the blend factor function
    /// making frequencies next to the lowest ones contribute more.
    ///
    /// Somewhat comparable to the Q factor of an equalizer node.
    ///
    /// Valid range:
    /// * 0.0 - base intensity and boosted intensity are linearly interpolated
    /// * 1.0 - all frequencies below maximum are at boosted intensity level
    pub low_frequency_boost_curvature: f32,

    /// Tightens how much the light scatters (default: 1.0).
    ///
    /// Valid range:
    /// * 0.0 - maximum scattering angle is 0 degrees (no scattering)
    /// * 1.0 - maximum scattering angle is 90 degrees
    pub high_pass_frequency: f32,

    /// Controls the threshold filter used for extracting the brightest regions from the input image
    /// before blurring them and compositing back onto the original image.
    ///
    /// Changing these settings creates a physically inaccurate image and makes it easy to make
    /// the final result look worse. However, they can be useful when emulating the 1990s-2000s game look.
    /// See [`BloomPrefilter`] for more information.
    pub prefilter: BloomPrefilter,

    /// Controls whether bloom textures
    /// are blended between or added to each other. Useful
    /// if image brightening is desired and a must-change
    /// if `prefilter` is used.
    ///
    /// # Recommendation
    /// Set to [`BloomCompositeMode::Additive`] if `prefilter` is
    /// configured in a non-energy-conserving way,
    /// otherwise set to [`BloomCompositeMode::EnergyConserving`].
    pub composite_mode: BloomCompositeMode,

    /// Maximum size of each dimension for the largest mipchain texture used in downscaling/upscaling.
    /// Only tweak if you are seeing visual artifacts.
    pub max_mip_dimension: u32,

    /// Amount to stretch the bloom on each axis. Artistic control, can be used to emulate
    /// anamorphic blur by using a large x-value. For large values, you may need to increase
    /// [`Bloom::max_mip_dimension`] to reduce sampling artifacts.
    pub scale: Vec2,
}

impl BloomSettings {
    const DEFAULT_MAX_MIP_DIMENSION: u32 = 512;

    /// The default bloom preset.
    ///
    /// This uses the [`EnergyConserving`](BloomCompositeMode::EnergyConserving) composite mode.
    pub const NATURAL: Self = Self {
        intensity: 0.15,
        low_frequency_boost: 0.7,
        low_frequency_boost_curvature: 0.95,
        high_pass_frequency: 1.0,
        prefilter: BloomPrefilter {
            threshold: 0.0,
            threshold_softness: 0.0,
        },
        composite_mode: BloomCompositeMode::EnergyConserving,
        max_mip_dimension: Self::DEFAULT_MAX_MIP_DIMENSION,
        scale: Vec2::ONE,
    };

    /// Emulates the look of stylized anamorphic bloom, stretched horizontally.
    pub const ANAMORPHIC: Self = Self {
        // The larger scale necessitates a larger resolution to reduce artifacts:
        max_mip_dimension: Self::DEFAULT_MAX_MIP_DIMENSION * 2,
        scale: Vec2::new(4.0, 1.0),
        ..Self::NATURAL
    };

    /// A preset that's similar to how older games did bloom.
    pub const OLD_SCHOOL: Self = Self {
        intensity: 0.05,
        low_frequency_boost: 0.7,
        low_frequency_boost_curvature: 0.95,
        high_pass_frequency: 1.0,
        prefilter: BloomPrefilter {
            threshold: 0.6,
            threshold_softness: 0.2,
        },
        composite_mode: BloomCompositeMode::Additive,
        max_mip_dimension: Self::DEFAULT_MAX_MIP_DIMENSION,
        scale: Vec2::ONE,
    };

    /// A preset that applies a very strong bloom, and blurs the whole screen.
    pub const SCREEN_BLUR: Self = Self {
        intensity: 1.0,
        low_frequency_boost: 0.0,
        low_frequency_boost_curvature: 0.0,
        high_pass_frequency: 1.0 / 3.0,
        prefilter: BloomPrefilter {
            threshold: 0.0,
            threshold_softness: 0.0,
        },
        composite_mode: BloomCompositeMode::EnergyConserving,
        max_mip_dimension: Self::DEFAULT_MAX_MIP_DIMENSION,
        scale: Vec2::ONE,
    };
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self::NATURAL
    }
}

/// Applies a threshold filter to the input image to extract the brightest
/// regions before blurring them and compositing back onto the original image.
/// These settings are useful when emulating the 1990s-2000s game look.
///
/// # Considerations
/// * Changing these settings creates a physically inaccurate image
/// * Changing these settings makes it easy to make the final result look worse
/// * Non-default prefilter settings should be used in conjunction with [`BloomCompositeMode::Additive`]
#[derive(Default, Clone, Debug)]
pub struct BloomPrefilter {
    /// Baseline of the quadratic threshold curve (default: 0.0).
    ///
    /// RGB values under the threshold curve will not contribute to the effect.
    pub threshold: f32,

    /// Controls how much to blend between the thresholded and non-thresholded colors (default: 0.0).
    ///
    /// 0.0 = Abrupt threshold, no blending
    /// 1.0 = Fully soft threshold
    ///
    /// Values outside of the range [0.0, 1.0] will be clamped.
    pub threshold_softness: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum BloomCompositeMode {
    EnergyConserving,
    Additive,
}

#[derive(Clone, Debug)]
pub struct Global {
    pub time: Duration,
    pub tonemap: TonemapSettings,
    pub bloom: BloomSettings,
    pub pipeline: Pipeline,
}

impl Default for Global {
    fn default() -> Self {
        Self {
            time: Duration::ZERO,
            tonemap: TonemapSettings::default(),
            bloom: BloomSettings::default(),
            pipeline: Pipeline::Mandlebulb,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Star {
    pub temperature: f32,
    pub sunspot_threshold: f32,
    pub sunspot_frequency: f32,
    pub granule_frequency: f32,
    pub granule_persistence: f32,
    pub color_shift: bool,
    pub time_scale: f32,
}

impl Star {
    pub fn sun() -> Self {
        Self::default()
    }

    pub fn with_temperature(self, temperature: f32) -> Self {
        Self {
            temperature,
            ..self
        }
    }

    pub fn with_time_scale(self, time_scale: f32) -> Self {
        Self { time_scale, ..self }
    }
}

impl Default for Star {
    fn default() -> Self {
        Self {
            temperature: 5778.0,
            sunspot_threshold: 0.2,
            sunspot_frequency: 5.0,
            granule_frequency: 40.0,
            granule_persistence: 0.7,
            color_shift: true,
            time_scale: 200.0,
        }
    }
}
