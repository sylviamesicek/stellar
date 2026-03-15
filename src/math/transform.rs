use std::ops::Mul;

use glam::{Mat3, Mat4, Quat, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Transform {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub const fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self::from_translation(Vec3::new(x, y, z))
    }

    pub const fn from_translation(translation: Vec3) -> Self {
        Transform {
            translation,
            ..Self::IDENTITY
        }
    }

    pub const fn from_rotation(rotation: Quat) -> Self {
        Transform {
            rotation,
            ..Self::IDENTITY
        }
    }

    pub const fn from_scale(scale: Vec3) -> Self {
        Transform {
            scale,
            ..Self::IDENTITY
        }
    }

    pub const fn with_uniform_scale(self, scale: f32) -> Self {
        Self {
            scale: glam::Vec3::new(scale, scale, scale),
            ..self
        }
    }

    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn to_scale_rotation_translation(&self) -> (Vec3, Quat, Vec3) {
        (self.scale, self.rotation, self.translation)
    }

    /// Get the unit vector in the local `X` direction.
    #[inline]
    pub fn local_x(&self) -> Vec3 {
        // Quat * unit vector is length 1
        self.rotation * Vec3::X
    }

    /// Equivalent to [`-local_x()`][Transform::local_x()]
    #[inline]
    pub fn left(&self) -> Vec3 {
        -self.local_x()
    }

    /// Equivalent to [`local_x()`][Transform::local_x()]
    #[inline]
    pub fn right(&self) -> Vec3 {
        self.local_x()
    }

    /// Get the unit vector in the local `Y` direction.
    #[inline]
    pub fn local_y(&self) -> Vec3 {
        // Quat * unit vector is length 1
        self.rotation * Vec3::Y
    }

    /// Equivalent to [`local_y()`][Transform::local_y]
    #[inline]
    pub fn up(&self) -> Vec3 {
        self.local_y()
    }

    /// Equivalent to [`-local_y()`][Transform::local_y]
    #[inline]
    pub fn down(&self) -> Vec3 {
        -self.local_y()
    }

    /// Get the unit vector in the local `Z` direction.
    #[inline]
    pub fn local_z(&self) -> Vec3 {
        // Quat * unit vector is length 1
        self.rotation * Vec3::Z
    }

    /// Equivalent to [`-local_z()`][Transform::local_z]
    #[inline]
    pub fn forward(&self) -> Vec3 {
        -self.local_z()
    }

    /// Equivalent to [`local_z()`][Transform::local_z]
    #[inline]
    pub fn back(&self) -> Vec3 {
        self.local_z()
    }

    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation = rotation * self.rotation;
    }

    /// Rotates this [`Transform`] around the given `axis` by `angle` (in radians).
    ///
    /// If this [`Transform`] has a parent, the `axis` is relative to the rotation of the parent.
    ///
    /// # Warning
    ///
    /// If you pass in an `axis` based on the current rotation (e.g. obtained via [`Transform::local_x`]),
    /// floating point errors can accumulate exponentially when applying rotations repeatedly this way. This will
    /// result in a denormalized rotation. In this case, it is recommended to normalize the [`Transform::rotation`] after
    /// each call to this method.
    #[inline]
    pub fn rotate_axis(&mut self, axis: Vec3, angle: f32) {
        #[cfg(debug_assertions)]
        assert_is_normalized(
            "The axis given to `Transform::rotate_axis` is not normalized. This may be a result of obtaining \
            the axis from the transform. See the documentation of `Transform::rotate_axis` for more details.",
            axis.length_squared(),
        );
        self.rotate(Quat::from_axis_angle(axis.into(), angle));
    }

    /// Rotates this [`Transform`] around the `X` axis by `angle` (in radians).
    ///
    /// If this [`Transform`] has a parent, the axis is relative to the rotation of the parent.
    #[inline]
    pub fn rotate_x(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_x(angle));
    }

    /// Rotates this [`Transform`] around the `Y` axis by `angle` (in radians).
    ///
    /// If this [`Transform`] has a parent, the axis is relative to the rotation of the parent.
    #[inline]
    pub fn rotate_y(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_y(angle));
    }

    /// Rotates this [`Transform`] around the `Z` axis by `angle` (in radians).
    ///
    /// If this [`Transform`] has a parent, the axis is relative to the rotation of the parent.
    #[inline]
    pub fn rotate_z(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_z(angle));
    }

    /// Rotates this [`Transform`] by the given `rotation`.
    ///
    /// The `rotation` is relative to this [`Transform`]'s current rotation.
    #[inline]
    pub fn rotate_local(&mut self, rotation: Quat) {
        self.rotation *= rotation;
    }

    /// Rotates this [`Transform`] around its local `X` axis by `angle` (in radians).
    #[inline]
    pub fn rotate_local_x(&mut self, angle: f32) {
        self.rotate_local(Quat::from_rotation_x(angle));
    }

    /// Rotates this [`Transform`] around its local `Y` axis by `angle` (in radians).
    #[inline]
    pub fn rotate_local_y(&mut self, angle: f32) {
        self.rotate_local(Quat::from_rotation_y(angle));
    }

    /// Rotates this [`Transform`] around its local `Z` axis by `angle` (in radians).
    #[inline]
    pub fn rotate_local_z(&mut self, angle: f32) {
        self.rotate_local(Quat::from_rotation_z(angle));
    }

    /// Translates this [`Transform`] around a `point` in space.
    ///
    /// If this [`Transform`] has a parent, the `point` is relative to the [`Transform`] of the parent.
    #[inline]
    pub fn translate_around(&mut self, point: Vec3, rotation: Quat) {
        self.translation = point + rotation * (self.translation - point);
    }

    /// Rotates this [`Transform`] around a `point` in space.
    ///
    /// If this [`Transform`] has a parent, the `point` is relative to the [`Transform`] of the parent.
    #[inline]
    pub fn rotate_around(&mut self, point: Vec3, rotation: Quat) {
        self.translate_around(point, rotation);
        self.rotate(rotation);
    }

    /// Returns this [`Transform`] with a new rotation so that [`Transform::forward`]
    /// points towards the `target` position and [`Transform::up`] points towards `up`.
    ///
    /// In some cases it's not possible to construct a rotation. Another axis will be picked in those cases:
    /// * if `target` is the same as the transform translation, `Vec3::Z` is used instead
    /// * if `up` fails converting to `Dir3` (e.g if it is `Vec3::ZERO`), `Dir3::Y` is used instead
    /// * if the resulting forward direction is parallel with `up`, an orthogonal vector is used as the "right" direction
    #[inline]
    #[must_use]
    pub fn looking_at(mut self, target: Vec3, up: Vec3) -> Self {
        self.look_at(target, up);
        self
    }

    /// Returns this [`Transform`] with a new rotation so that [`Transform::forward`]
    /// points in the given `direction` and [`Transform::up`] points towards `up`.
    ///
    /// In some cases it's not possible to construct a rotation. Another axis will be picked in those cases:
    /// * if `direction` fails converting to `Dir3` (e.g if it is `Vec3::ZERO`), `Dir3::Z` is used instead
    /// * if `up` fails converting to `Dir3`, `Dir3::Y` is used instead
    /// * if `direction` is parallel with `up`, an orthogonal vector is used as the "right" direction
    #[inline]
    #[must_use]
    pub fn looking_to(mut self, direction: Vec3, up: Vec3) -> Self {
        self.look_to(direction, up);
        self
    }

    /// Rotates this [`Transform`] so that [`Transform::forward`] points towards the `target` position,
    /// and [`Transform::up`] points towards `up`.
    ///
    /// In some cases it's not possible to construct a rotation. Another axis will be picked in those cases:
    /// * if `target` is the same as the transform translation, `Vec3::Z` is used instead
    /// * if `up` fails converting to `Dir3` (e.g if it is `Vec3::ZERO`), `Dir3::Y` is used instead
    /// * if the resulting forward direction is parallel with `up`, an orthogonal vector is used as the "right" direction
    #[inline]
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        self.look_to(target - self.translation, up);
    }

    /// Rotates this [`Transform`] so that [`Transform::forward`] points in the given `direction`
    /// and [`Transform::up`] points towards `up`.
    ///
    /// In some cases it's not possible to construct a rotation. Another axis will be picked in those cases:
    /// * if `direction` fails converting to `Dir3` (e.g if it is `Vec3::ZERO`), `Dir3::NEG_Z` is used instead
    /// * if `up` fails converting to `Dir3`, `Dir3::Y` is used instead
    /// * if `direction` is parallel with `up`, an orthogonal vector is used as the "right" direction
    #[inline]
    pub fn look_to(&mut self, direction: Vec3, mut up: Vec3) {
        let mut back = -direction;
        if direction == Vec3::ZERO {
            back = Vec3::NEG_Z;
        }

        debug_assert!((up.length_squared() - 1.0).abs() <= 1e-3);

        if up == Vec3::ZERO {
            up = Vec3::Y;
        }

        let right = up
            .cross(back)
            .try_normalize()
            .unwrap_or_else(|| up.any_orthogonal_vector());
        let up = back.cross(right);

        self.rotation = Quat::from_mat3(&Mat3::from_cols(right, up, back));
    }

    /// Multiplies `self` with `transform` component by component, returning the
    /// resulting [`Transform`]
    #[inline]
    #[must_use]
    pub fn mul_transform(&self, transform: Transform) -> Self {
        let translation = self.transform_point(transform.translation);
        let rotation = self.rotation * transform.rotation;
        let scale = self.scale * transform.scale;
        Transform {
            translation,
            rotation,
            scale,
        }
    }

    /// Transforms the given `point`, applying scale, rotation and translation.
    ///
    /// If this [`Transform`] has an ancestor entity with a [`Transform`] component,
    /// [`Transform::transform_point`] will transform a point in local space into its
    /// parent transform's space.
    ///
    /// If this [`Transform`] does not have a parent, [`Transform::transform_point`] will
    /// transform a point in local space into worldspace coordinates.
    ///
    /// If you always want to transform a point in local space to worldspace, or if you need
    /// the inverse transformations, see [`GlobalTransform::transform_point()`].
    #[inline]
    pub fn transform_point(&self, mut point: Vec3) -> Vec3 {
        point = self.scale * point;
        point = self.rotation * point;
        point += self.translation;
        point
    }

    /// Returns `true` if, and only if, translation, rotation and scale all are
    /// finite. If any of them contains a `NaN`, positive or negative infinity,
    /// this will return `false`.
    #[inline]
    #[must_use]
    pub fn is_finite(&self) -> bool {
        self.translation.is_finite() && self.rotation.is_finite() && self.scale.is_finite()
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mul<Transform> for Transform {
    type Output = Transform;

    fn mul(self, transform: Transform) -> Self::Output {
        self.mul_transform(transform)
    }
}

impl Mul<Vec3> for Transform {
    type Output = Vec3;

    fn mul(self, value: Vec3) -> Self::Output {
        self.transform_point(value)
    }
}

/// Checks that a vector with the given squared length is normalized.
///
/// Warns for small error with a length threshold of approximately `1e-4`,
/// and panics for large error with a length threshold of approximately `1e-2`.
#[cfg(debug_assertions)]
fn assert_is_normalized(message: &str, length_squared: f32) {
    use std::eprintln;

    let length_error_squared = (length_squared - 1.0).abs();

    // Panic for large error and warn for slight error.
    if length_error_squared > 2e-2 || length_error_squared.is_nan() {
        // Length error is approximately 1e-2 or more.
        panic!("Error: {message}",);
    } else if length_error_squared > 2e-4 {
        // Length error is approximately 1e-4 or more.
        #[expect(clippy::print_stderr, reason = "Allowed behind `std` feature gate.")]
        {
            eprintln!("Warning: {message}",);
        }
    }
}
