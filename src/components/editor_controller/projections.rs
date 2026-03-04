//! Configurable options for the challenge of working with orthographic cameras.

use glam::Vec3;

use crate::{
    components::editor_controller::component::EditorCam,
    math::{Projection, Transform},
};

use super::motion::CurrentMotion;

/// Settings used when the [`EditorCam`] has a perspective [`Projection`].
#[derive(Debug, Clone)]
pub struct PerspectiveSettings {
    /// Limits the near clipping plane to always fit inside this range.
    ///
    /// The camera controller will try to make the near clipping plane smaller when you zoom in to
    /// ensure the anchor (the thing you are zooming into) is always within the view frustum
    /// (visible), bounded by this limit.
    ///
    /// Unless the camera is zoomed very close to something, it will spend most of the time at the
    /// high end of this limit - you should treat that like the default near clipping plane. Bevy
    /// defaults to `0.1`, and you should probably use that too unless you have a very good reason
    /// not to. Many rendering effects that rely on depth can break down if the clipping plane is
    /// very far from `0.1`.
    pub near_clip_limits: std::ops::Range<f32>,
    /// When computing the near plane position, the anchor depth is multiplied by this value to
    /// determine the new near clip position. This should be smaller than one, to ensure that the
    /// object you are looking at, which will be located at the anchor position, is bot being
    /// clipped. Some parts of the object may protrude toward the camera, which is what necessitates
    /// this.
    pub near_clip_multiplier: f32,
}

impl Default for PerspectiveSettings {
    fn default() -> Self {
        Self {
            near_clip_limits: 1e-9..f32::INFINITY,
            near_clip_multiplier: 0.05,
        }
    }
}

/// Updates perspective projection properties of editor cameras.
pub fn update_perspective(controller: &mut EditorCam, projection: &mut Projection) {
    let Projection::Perspective(ref mut perspective) = *projection else {
        return;
    };
    let limits = controller.perspective.near_clip_limits.clone();
    let multiplier = controller.perspective.near_clip_multiplier;
    perspective.near =
        (controller.last_anchor_depth.abs() as f32 * multiplier).clamp(limits.start, limits.end);
}

/// Settings used when the [`EditorCam`] has an orthographic [`Projection`].
#[derive(Debug, Clone)]
pub struct OrthographicSettings {
    /// The camera's near clipping plane will move closer and farther from the anchor point during
    /// zoom to maximize precision. The position of the near plane is based on the orthographic
    /// projection `scale`, multiplied by this value.
    ///
    /// To maximize depth precision, make this as small ap possible. If the value is too large,
    /// depth-based effects like SSAO will break down. If the value is too small, objects that
    /// should be visible will be clipped. Ideally, the clipping planes should scale with the scene
    /// geometry and camera frustum to tightly bound the visible scene, but this is not yet
    /// implemented.
    pub scale_to_near_clip: f32,
    /// Limits the distance the near clip plane can be to the anchor. The low limit is useful to
    /// prevent geometry clipping when zooming in, while the high limit is useful to prevent the
    /// camera moving too far away from the anchor, causing precision issues.
    pub near_clip_limits: std::ops::Range<f32>,
    /// The far plane is placed opposite the anchor from the near plane, at this multiple of the
    /// distance from the near plane to the anchor. Setting this to 1.0 means the camera frustum is
    /// centered on the anchor. It might be desirable to make this larger to prevent things in the
    /// background from disappearing when zooming in.
    pub far_clip_multiplier: f32,
}

impl Default for OrthographicSettings {
    fn default() -> Self {
        Self {
            scale_to_near_clip: 1_000_000.0,
            near_clip_limits: 1.0..1_000_000.0,
            far_clip_multiplier: 1.0,
        }
    }
}

/// Update the ortho camera projection and position based on the [`OrthographicSettings`].
pub fn update_orthographic(
    controller: &mut EditorCam,
    projection: &mut Projection,
    transform: &mut Transform,
) {
    let Projection::Orthographic(ref mut orthographic) = *projection else {
        return;
    };

    let anchor_dist = controller.last_anchor_depth().abs() as f32;
    let target_dist = (controller.orthographic.scale_to_near_clip * orthographic.scale).clamp(
        controller.orthographic.near_clip_limits.start,
        controller.orthographic.near_clip_limits.end,
    );

    let forward_amount = anchor_dist - target_dist;
    let movement = transform.forward() * forward_amount;

    if movement != Vec3::ZERO {
        transform.translation += movement;
    }

    controller.last_anchor_depth += forward_amount as f64;
    if let CurrentMotion::UserControlled { ref mut anchor, .. } = controller.current_motion {
        anchor.z += forward_amount as f64;
    }

    orthographic.near = 0.0;
    orthographic.far = anchor_dist * (1.0 + controller.orthographic.far_clip_multiplier);
}
