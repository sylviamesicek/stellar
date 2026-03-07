use crate::{
    components::Camera,
    math::{Projection, Transform},
};
use egui::InputState;
use glam::{Vec2, Vec3};
use std::{f32::consts::PI, time::Duration};

mod traits;
mod util;

use traits::OptionalClamp as _;

pub struct PanOrbitController {
    /// The point to orbit around, and what the camera looks at. Updated automatically.
    /// If you want to change the focus programmatically after initialization, set `target_focus`
    /// instead.
    /// Defaults to `Vec3::ZERO`.
    pub focus: Vec3,
    /// The radius of the orbit, or the distance from the `focus` point.
    /// For orthographic projection, this is ignored, and the projection's `scale` is used instead.
    /// If set to `None`, it will be calculated from the camera's current position during
    /// initialization.
    /// Automatically updated.
    /// Defaults to `None`.
    pub radius: Option<f32>,
    /// Rotation in radians around the global Y axis (longitudinal). Updated automatically.
    /// If both `yaw` and `pitch` are `0.0`, then the camera will be looking forward, i.e. in
    /// the `Vec3::NEG_Z` direction, with up being `Vec3::Y`.
    /// If set to `None`, it will be calculated from the camera's current position during
    /// initialization.
    /// You should not update this after initialization - use `target_yaw` instead.
    /// Defaults to `None`.
    pub yaw: Option<f32>,
    /// Rotation in radians around the local X axis (latitudinal). Updated automatically.
    /// If both `yaw` and `pitch` are `0.0`, then the camera will be looking forward, i.e. in
    /// the `Vec3::NEG_Z` direction, with up being `Vec3::Y`.
    /// If set to `None`, it will be calculated from the camera's current position during
    /// initialization.
    /// You should not update this after initialization - use `target_pitch` instead.
    /// Defaults to `None`.
    pub pitch: Option<f32>,
    /// The target focus point. The camera will smoothly transition to this value. Updated
    /// automatically, but you can also update it manually to control the camera independently of
    /// the mouse controls, e.g. with the keyboard.
    /// Defaults to `Vec3::ZERO`.
    pub target_focus: Vec3,
    /// The target yaw value. The camera will smoothly transition to this value. Updated
    /// automatically, but you can also update it manually to control the camera independently of
    /// the mouse controls, e.g. with the keyboard.
    /// Defaults to `0.0`.
    pub target_yaw: f32,
    /// The target pitch value. The camera will smoothly transition to this value Updated
    /// automatically, but you can also update it manually to control the camera independently of
    /// the mouse controls, e.g. with the keyboard.
    /// Defaults to `0.0`.
    pub target_pitch: f32,
    /// The target radius value. The camera will smoothly transition to this value. Updated
    /// automatically, but you can also update it manually to control the camera independently of
    /// the mouse controls, e.g. with the keyboard.
    /// Defaults to `1.0`.
    pub target_radius: f32,
    /// Upper limit on the `yaw` value, in radians. Use this to restrict the maximum rotation
    /// around the global Y axis.
    /// Defaults to `None`.
    pub yaw_upper_limit: Option<f32>,
    /// Lower limit on the `yaw` value, in radians. Use this to restrict the maximum rotation
    /// around the global Y axis.
    /// Defaults to `None`.
    pub yaw_lower_limit: Option<f32>,
    /// Upper limit on the `pitch` value, in radians. Use this to restrict the maximum rotation
    /// around the local X axis.
    /// Defaults to `None`.
    pub pitch_upper_limit: Option<f32>,
    /// Lower limit on the `pitch` value, in radians. Use this to restrict the maximum rotation
    /// around the local X axis.
    /// Defaults to `None`.
    pub pitch_lower_limit: Option<f32>,
    /// The origin for a shape to restrict the cameras `focus` position.
    /// Defaults to `Vec3::ZERO`.
    pub focus_bounds_origin: Vec3,
    // /// The shape (Sphere or Cuboid) that the `focus` is restricted by. Centered on the
    // /// `focus_bounds_origin`.
    // /// Defaults to `None`.
    // pub focus_bounds_shape: Option<FocusBoundsShape>,
    /// Upper limit on the zoom. This applies to `radius`, in the case of using a perspective
    /// camera, or the projection's scale in the case of using an orthographic camera.
    /// Defaults to `None`.
    pub zoom_upper_limit: Option<f32>,
    /// Lower limit on the zoom. This applies to `radius`, in the case of using a perspective
    /// camera, or the projection's scale in the case of using an orthographic camera.
    /// Should always be >0 otherwise you'll get stuck at 0.
    /// Defaults to `0.05`.
    pub zoom_lower_limit: f32,
    /// The sensitivity of the orbiting motion. A value of `0.0` disables orbiting.
    /// Defaults to `1.0`.
    pub orbit_sensitivity: f32,
    /// How much smoothing is applied to the orbit motion. A value of `0.0` disables smoothing,
    /// so there's a 1:1 mapping of input to camera position. A value of `1.0` is infinite
    /// smoothing.
    /// Defaults to `0.8`.
    pub orbit_smoothness: f32,
    /// The sensitivity of the panning motion. A value of `0.0` disables panning.
    /// Defaults to `1.0`.
    pub pan_sensitivity: f32,
    /// How much smoothing is applied to the panning motion. A value of `0.0` disables smoothing,
    /// so there's a 1:1 mapping of input to camera position. A value of `1.0` is infinite
    /// smoothing.
    /// Defaults to `0.6`.
    pub pan_smoothness: f32,
    /// The sensitivity of moving the camera closer or further way using the scroll wheel.
    /// A value of `0.0` disables zooming.
    /// Defaults to `1.0`.
    pub zoom_sensitivity: f32,
    /// How much smoothing is applied to the zoom motion. A value of `0.0` disables smoothing,
    /// so there's a 1:1 mapping of input to camera position. A value of `1.0` is infinite
    /// smoothing.
    /// Defaults to `0.8`.
    /// Note that this setting does not apply to pixel-based scroll events, as they are typically
    /// already smooth. It only applies to line-based scroll events.
    pub zoom_smoothness: f32,
    /// Button used to orbit the camera.
    /// Defaults to `Button::Left`.
    pub button_orbit: egui::PointerButton,
    /// Button used to pan the camera.
    /// Defaults to `Button::Right`.
    pub button_pan: egui::PointerButton,
    /// Button used to zoom the camera, by holding it down and moving the mouse forward and back.
    /// Defaults to `None`.
    pub button_zoom: Option<egui::PointerButton>,
    /// The sensitivity of trackpad gestures when using `BlenderLike` behavior. A value of `0.0`
    /// effectively disables trackpad orbit/pan functionality. This applies to both orbit and pan.
    /// operations when using a trackpad with the `BlenderLike` behavior mode.
    /// Defaults to `1.0`.
    pub trackpad_sensitivity: f32,
    /// Whether to reverse the zoom direction. This applies to the button-based zoom `button_zoom`
    /// as well. If you want button zoom to remain the same, set `button_zoom_reverse` to `true`.
    /// Defaults to `false`.
    pub reversed_zoom: bool,
    /// Whether the zoom direction when using `button_zoom` is reversed.
    /// Defaults to `false`.
    pub reversed_button_zoom: bool,
    /// Whether the camera is currently upside down. Updated automatically.
    /// This is used to determine which way to orbit, because it's more intuitive to reverse the
    /// orbit direction when upside down.
    /// Should not be set manually unless you know what you're doing.
    /// Defaults to `false` (but will be updated immediately).
    pub is_upside_down: bool,
    /// Whether to allow the camera to go upside down.
    /// Defaults to `false`.
    pub allow_upside_down: bool,
    /// If `false`, disable control of the camera. Defaults to `true`.
    pub enabled: bool,
    /// Whether `PanOrbitCamera` has been initialized with the initial config.
    /// Set to `true` if you want the camera to smoothly animate to its initial position.
    /// Defaults to `false`.
    pub initialized: bool,
    /// Whether to update the camera's transform regardless of whether there are any changes/input.
    /// Set this to `true` if you want to modify values directly.
    /// This will be automatically set back to `false` after one frame.
    /// Defaults to `false`.
    pub force_update: bool,
    /// Axis order definition. This can be used to e.g. define a different default
    /// up direction. The default up is Y, but if you want the camera rotated.
    /// The axis can be switched.
    /// Defaults to `[Vec3::X, Vec3::Y, Vec3::Z]`.
    pub axis: [Vec3; 3],
}

impl Default for PanOrbitController {
    fn default() -> Self {
        PanOrbitController {
            focus: Vec3::ZERO,
            target_focus: Vec3::ZERO,
            radius: None,
            is_upside_down: false,
            allow_upside_down: false,
            orbit_sensitivity: 1.0,
            orbit_smoothness: 0.1,
            pan_sensitivity: 1.0,
            pan_smoothness: 0.02,
            zoom_sensitivity: 1.0,
            zoom_smoothness: 0.1,
            button_orbit: egui::PointerButton::Primary,
            button_pan: egui::PointerButton::Secondary,
            button_zoom: None,
            reversed_button_zoom: false,
            trackpad_sensitivity: 1.0,
            reversed_zoom: false,
            enabled: true,
            yaw: None,
            pitch: None,
            target_yaw: 0.0,
            target_pitch: 0.0,
            target_radius: 1.0,
            initialized: false,
            yaw_upper_limit: None,
            yaw_lower_limit: None,
            pitch_upper_limit: None,
            pitch_lower_limit: None,
            focus_bounds_origin: Vec3::ZERO,
            // focus_bounds_shape: None,
            zoom_upper_limit: None,
            zoom_lower_limit: 0.05,
            force_update: false,
            axis: [Vec3::X, Vec3::Y, Vec3::Z],
        }
    }
}

/// Main system for processing input and converting to transformations
pub fn update_pan_orbit_camera(
    state: &InputState,
    duration: Duration,
    transform: &mut Transform,
    camera: &mut Camera,
    controller: &mut PanOrbitController,
) {
    let delta = duration.as_secs_f32();

    // Closures that apply limits to the yaw, pitch, and zoom values
    let apply_zoom_limits = {
        let zoom_upper_limit = controller.zoom_upper_limit;
        let zoom_lower_limit = controller.zoom_lower_limit;
        move |zoom: f32| zoom.clamp_optional(Some(zoom_lower_limit), zoom_upper_limit)
    };

    let apply_yaw_limits = {
        let yaw_upper_limit = controller.yaw_upper_limit;
        let yaw_lower_limit = controller.yaw_lower_limit;
        move |yaw: f32| yaw.clamp_optional(yaw_lower_limit, yaw_upper_limit)
    };

    let apply_pitch_limits = {
        let pitch_upper_limit = controller.pitch_upper_limit;
        let pitch_lower_limit = controller.pitch_lower_limit;
        move |pitch: f32| pitch.clamp_optional(pitch_lower_limit, pitch_upper_limit)
    };

    let apply_focus_limits = {
        let _origin = controller.focus_bounds_origin;
        move |focus| focus

        // let shape = pan_orbit.focus_bounds_shape;

        // move |focus: Vec3| {
        //     let Some(shape) = shape else {
        //         return focus;
        //     };

        //     match shape {
        //         FocusBoundsShape::Cuboid(shape) => shape.closest_point(focus - origin) + origin,
        //         FocusBoundsShape::Sphere(shape) => shape.closest_point(focus - origin) + origin,
        //     }
        // }
    };

    if !controller.initialized {
        // Calculate yaw, pitch, and radius from the camera's position. If user sets all
        // these explicitly, this calculation is wasted, but that's okay since it will only run
        // once on init.
        let (yaw, pitch, radius) = util::calculate_from_translation_and_focus(
            transform.translation,
            controller.focus,
            controller.axis,
        );
        let &mut mut yaw = controller.yaw.get_or_insert(yaw);
        let &mut mut pitch = controller.pitch.get_or_insert(pitch);
        let &mut mut radius = controller.radius.get_or_insert(radius);
        let mut focus = controller.focus;

        // Apply limits
        yaw = apply_yaw_limits(yaw);
        pitch = apply_pitch_limits(pitch);
        radius = apply_zoom_limits(radius);
        focus = apply_focus_limits(focus);

        // Set initial values
        controller.yaw = Some(yaw);
        controller.pitch = Some(pitch);
        controller.radius = Some(radius);
        controller.target_yaw = yaw;
        controller.target_pitch = pitch;
        controller.target_radius = radius;
        controller.target_focus = focus;

        util::update_orbit_transform(
            yaw,
            pitch,
            radius,
            focus,
            transform,
            &mut camera.projection,
            controller.axis,
        );

        controller.initialized = true;
    }

    // 1 - Get Input

    let mut orbit = Vec2::ZERO;
    let mut pan = Vec2::ZERO;
    let mut scroll = 0.0f32;
    let mut orbit_button_changed = false;

    // The reason we only skip getting input if the camera is inactive/disabled is because
    // it might still be moving (lerping towards target values) when the user is not
    // actively controlling it.
    if controller.enabled {
        // Collect input deltas
        let pointer_delta = state.pointer.delta();
        let pointer_delta = Vec2::new(pointer_delta[0], pointer_delta[1]);

        let mut input_orbit = Vec2::ZERO;
        let mut input_pan = Vec2::ZERO;

        // Handle mouse movement for orbiting and panning
        if state.pointer.button_down(controller.button_orbit) {
            input_orbit += pointer_delta;
        } else if state.pointer.button_down(controller.button_pan) {
            input_pan += pointer_delta;
        }

        orbit_button_changed = state.pointer.button_pressed(controller.button_orbit)
            || state.pointer.button_released(controller.button_orbit);

        orbit = input_orbit * controller.orbit_sensitivity;
        pan = input_pan * controller.pan_sensitivity;
        // scroll = state.zoom_delta() * controller.zoom_sensitivity;
    }

    // 2 - Process input into target yaw/pitch, or focus, radius

    // Only check for upside down when orbiting started or ended this frame,
    // so we don't reverse the yaw direction while the user is still dragging
    if orbit_button_changed {
        let world_up = controller.axis[1];
        controller.is_upside_down = transform.up().dot(world_up) < 0.0;
    }

    let mut has_moved = false;
    if orbit.length_squared() > 0.0 {
        // Use window size for rotation otherwise the sensitivity
        // is far too high for small viewports
        let win_size: [_; 2] = std::array::from_fn(|i| camera.physical_size()[i] as f32);

        let delta_x = {
            let delta = orbit.x / win_size[0] * PI * 2.0;
            if controller.is_upside_down {
                -delta
            } else {
                delta
            }
        };
        let delta_y = orbit.y / win_size[1] * PI;
        controller.target_yaw -= delta_x;
        controller.target_pitch += delta_y;

        has_moved = true;
    }
    if pan.length_squared() > 0.0 {
        let win_size: [_; 2] = std::array::from_fn(|i| camera.physical_size()[i] as f32);
        let mut multiplier = 1.0;
        match &camera.projection {
            Projection::Perspective(p) => {
                pan *= Vec2::new(p.fov * p.aspect_ratio, p.fov) / win_size[0];
                // Make panning proportional to distance away from focus point
                if let Some(radius) = controller.radius {
                    multiplier = radius;
                }
            }
            Projection::Orthographic(p) => {
                pan *= Vec2::new(p.area.width(), p.area.height()) / win_size[1];
            }
        }
        // Translate by local axes
        let right = transform.rotation * controller.axis[0] * -pan.x;
        let up = transform.rotation * controller.axis[1] * pan.y;
        let translation = (right + up) * multiplier;
        controller.target_focus += translation;
        has_moved = true;

        // Make panning distance independent of resolution and FOV,
    }
    if scroll.abs() > 0.0 {
        // Calculate the impact of scrolling on the reference value
        let line_delta = -scroll * (controller.target_radius) * 0.2;
        let pixel_delta = -scroll * (controller.target_radius) * 0.2;

        // Update the target value
        controller.target_radius += line_delta + pixel_delta;

        // If it is pixel-based scrolling, add it directly to the current value
        controller.radius = controller
            .radius
            .map(|value| apply_zoom_limits(value + pixel_delta));

        has_moved = true;
    }

    // 3 - Apply constraints

    controller.target_yaw = apply_yaw_limits(controller.target_yaw);
    controller.target_pitch = apply_pitch_limits(controller.target_pitch);
    controller.target_radius = apply_zoom_limits(controller.target_radius);
    controller.target_focus = apply_focus_limits(controller.target_focus);

    if !controller.allow_upside_down {
        controller.target_pitch = controller.target_pitch.clamp(-PI / 2.0, PI / 2.0);
    }

    // 4 - Update the camera's transform based on current values

    // let delta = if pan_orbit.use_real_time {
    //     time_real.delta_secs()
    // } else {
    //     time_virt.delta_secs()
    // };

    if let (Some(yaw), Some(pitch), Some(radius)) =
        (controller.yaw, controller.pitch, controller.radius)
    {
        if has_moved
                // For smoothed values, we must check whether current value is different from target
                // value. If we only checked whether the values were non-zero this frame, then
                // the camera would instantly stop moving as soon as you stopped moving it, instead
                // of smoothly stopping
                || controller.target_yaw != yaw
                || controller.target_pitch != pitch
                || controller.target_radius != radius
                || controller.target_focus != controller.focus
                || controller.force_update
        {
            // Interpolate towards the target values
            let new_yaw = util::lerp_and_snap_f32(
                yaw,
                controller.target_yaw,
                controller.orbit_smoothness,
                delta,
            );
            let new_pitch = util::lerp_and_snap_f32(
                pitch,
                controller.target_pitch,
                controller.orbit_smoothness,
                delta,
            );
            let new_radius = util::lerp_and_snap_f32(
                radius,
                controller.target_radius,
                controller.zoom_smoothness,
                delta,
            );
            let new_focus = util::lerp_and_snap_vec3(
                controller.focus,
                controller.target_focus,
                controller.pan_smoothness,
                delta,
            );

            util::update_orbit_transform(
                new_yaw,
                new_pitch,
                new_radius,
                new_focus,
                transform,
                &mut camera.projection,
                controller.axis,
            );

            // Update the current values
            controller.yaw = Some(new_yaw);
            controller.pitch = Some(new_pitch);
            controller.radius = Some(new_radius);
            controller.focus = new_focus;
            controller.force_update = false;
        }
    }
}
