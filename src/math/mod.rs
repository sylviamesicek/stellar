pub use bevy_math::*;
pub use bevy_transform::components::Transform;
pub use bevy_transform::traits::TransformPoint;

mod projection;

pub use projection::{CameraProjection, OrthographicProjection, PerspectiveProjection, Projection};

/// A region of 3D space, specifically an open set whose border is a bisecting 2D plane.
///
/// This bisecting plane partitions 3D space into two infinite regions,
/// the half-space is one of those regions and excludes the bisecting plane.
///
/// Each instance of this type is characterized by:
/// - the bisecting plane's unit normal, normalized and pointing "inside" the half-space,
/// - the signed distance along the normal from the bisecting plane to the origin of 3D space.
///
/// The distance can also be seen as:
/// - the distance along the inverse of the normal from the origin of 3D space to the bisecting plane,
/// - the opposite of the distance along the normal from the origin of 3D space to the bisecting plane.
///
/// Any point `p` is considered to be within the `HalfSpace` when the length of the projection
/// of p on the normal is greater or equal than the opposite of the distance,
/// meaning: if the equation `normal.dot(p) + distance > 0.` is satisfied.
///
/// For example, the half-space containing all the points with a z-coordinate lesser
/// or equal than `8.0` would be defined by: `HalfSpace::new(Vec3::NEG_Z.extend(-8.0))`.
/// It includes all the points from the bisecting plane towards `NEG_Z`, and the distance
/// from the plane to the origin is `-8.0` along `NEG_Z`.
///
/// It is used to define a [`Frustum`], but is also a useful mathematical primitive for rendering tasks such as  light computation.
#[derive(Clone, Copy, Debug, Default)]
pub struct HalfSpace {
    normal_d: Vec4,
}

impl HalfSpace {
    /// Constructs a `HalfSpace` from a 4D vector whose first 3 components
    /// represent the bisecting plane's unit normal, and the last component is
    /// the signed distance along the normal from the plane to the origin.
    /// The constructor ensures the normal vector is normalized and the distance is appropriately scaled.
    #[inline]
    pub fn new(normal_d: Vec4) -> Self {
        Self {
            normal_d: normal_d * normal_d.xyz().length_recip(),
        }
    }

    /// Returns the unit normal vector of the bisecting plane that characterizes the `HalfSpace`.
    #[inline]
    pub fn normal(&self) -> Vec3A {
        Vec3A::from_vec4(self.normal_d)
    }

    /// Returns the signed distance from the bisecting plane to the origin along
    /// the plane's unit normal vector.
    #[inline]
    pub fn d(&self) -> f32 {
        self.normal_d.w
    }

    /// Returns the bisecting plane's unit normal vector and the signed distance
    /// from the plane to the origin.
    #[inline]
    pub fn normal_d(&self) -> Vec4 {
        self.normal_d
    }
}

/// A region of 3D space defined by the intersection of 6 [`HalfSpace`]s.
///
/// Frustums are typically an apex-truncated square pyramid (a pyramid without the top) or a cuboid.
///
/// Half spaces are ordered left, right, top, bottom, near, far. The normal vectors
/// of the half-spaces point towards the interior of the frustum.
///
/// A frustum component is used on an entity with a [`Camera`] component to
/// determine which entities will be considered for rendering by this camera.
/// All entities with an [`Aabb`] component that are not contained by (or crossing
/// the boundary of) the frustum will not be rendered, and not be used in rendering computations.
///
/// This process is called frustum culling, and entities can opt out of it using
/// the [`NoFrustumCulling`] component.
///
/// The frustum component is typically added automatically for cameras, either [`Camera2d`] or [`Camera3d`].
/// It is usually updated automatically by [`update_frusta`] from the
/// [`CameraProjection`] component and [`GlobalTransform`] of the camera entity.
///
/// [`Camera`]: crate::Camera
/// [`NoFrustumCulling`]: crate::visibility::NoFrustumCulling
/// [`update_frusta`]: crate::visibility::update_frusta
/// [`CameraProjection`]: crate::CameraProjection
/// [`GlobalTransform`]: bevy_transform::components::GlobalTransform
/// [`Camera2d`]: crate::Camera2d
/// [`Camera3d`]: crate::Camera3d
#[derive(Clone, Copy, Debug, Default)]
pub struct Frustum {
    pub half_spaces: [HalfSpace; 6],
}

impl Frustum {
    pub const NEAR_PLANE_IDX: usize = 4;
    const FAR_PLANE_IDX: usize = 5;
    const INACTIVE_HALF_SPACE: Vec4 = Vec4::new(0.0, 0.0, 0.0, f32::INFINITY);

    /// Returns a frustum derived from `clip_from_world`.
    #[inline]
    pub fn from_clip_from_world(clip_from_world: &Mat4) -> Self {
        let mut frustum = Frustum::from_clip_from_world_no_far(clip_from_world);
        frustum.half_spaces[Self::FAR_PLANE_IDX] = HalfSpace::new(clip_from_world.row(2));
        frustum
    }

    /// Returns a frustum derived from `clip_from_world`,
    /// but with a custom far plane.
    #[inline]
    pub fn from_clip_from_world_custom_far(
        clip_from_world: &Mat4,
        view_translation: &Vec3,
        view_backward: &Vec3,
        far: f32,
    ) -> Self {
        let mut frustum = Frustum::from_clip_from_world_no_far(clip_from_world);
        let far_center = *view_translation - far * *view_backward;
        frustum.half_spaces[Self::FAR_PLANE_IDX] =
            HalfSpace::new(view_backward.extend(-view_backward.dot(far_center)));
        frustum
    }

    // NOTE: This approach of extracting the frustum half-space from the view
    // projection matrix is from Foundations of Game Engine Development 2
    // Rendering by Lengyel.
    /// Returns a frustum derived from `view_projection`,
    /// without a far plane.
    fn from_clip_from_world_no_far(clip_from_world: &Mat4) -> Self {
        let row0 = clip_from_world.row(0);
        let row1 = clip_from_world.row(1);
        let row2 = clip_from_world.row(2);
        let row3 = clip_from_world.row(3);

        Self {
            half_spaces: [
                HalfSpace::new(row3 + row0),
                HalfSpace::new(row3 - row0),
                HalfSpace::new(row3 + row1),
                HalfSpace::new(row3 - row1),
                HalfSpace::new(row3 + row2),
                HalfSpace::new(Self::INACTIVE_HALF_SPACE),
            ],
        }
    }

    // /// Checks if a sphere intersects the frustum.
    // #[inline]
    // pub fn intersects_sphere(&self, sphere: &Sphere, intersect_far: bool) -> bool {
    //     let sphere_center = sphere.center.extend(1.0);
    //     let max = if intersect_far {
    //         Self::FAR_PLANE_IDX
    //     } else {
    //         Self::NEAR_PLANE_IDX
    //     };
    //     for half_space in &self.half_spaces[..=max] {
    //         if half_space.normal_d().dot(sphere_center) + sphere.radius <= 0.0 {
    //             return false;
    //         }
    //     }
    //     true
    // }

    // /// Checks if an Oriented Bounding Box (obb) intersects the frustum.
    // #[inline]
    // pub fn intersects_obb(
    //     &self,
    //     aabb: &Aabb,
    //     world_from_local: &Affine3A,
    //     intersect_near: bool,
    //     intersect_far: bool,
    // ) -> bool {
    //     let aabb_center_world = world_from_local.transform_point3a(aabb.center).extend(1.0);

    //     for (idx, half_space) in self.half_spaces.into_iter().enumerate() {
    //         if (idx == Self::NEAR_PLANE_IDX && !intersect_near)
    //             || (idx == Self::FAR_PLANE_IDX && !intersect_far)
    //         {
    //             continue;
    //         }
    //         let p_normal = half_space.normal();
    //         let relative_radius = aabb.relative_radius(&p_normal, &world_from_local.matrix3);
    //         if half_space.normal_d().dot(aabb_center_world) + relative_radius <= 0.0 {
    //             return false;
    //         }
    //     }
    //     true
    // }

    // /// Optimized version of [`Frustum::intersects_obb`]
    // /// where the transform is [`Affine3A::IDENTITY`] and both `intersect_near` and `intersect_far` are `true`.
    // #[inline]
    // pub fn intersects_obb_identity(&self, aabb: &Aabb) -> bool {
    //     let aabb_center_world = aabb.center.extend(1.0);
    //     for half_space in self.half_spaces.iter() {
    //         let p_normal = half_space.normal();
    //         let relative_radius = aabb.half_extents.abs().dot(p_normal.abs());
    //         if half_space.normal_d().dot(aabb_center_world) + relative_radius <= 0.0 {
    //             return false;
    //         }
    //     }
    //     true
    // }

    // /// Check if the frustum contains the entire Axis-Aligned Bounding Box (AABB).
    // /// Referenced from: [Frustum Culling](https://learnopengl.com/Guest-Articles/2021/Scene/Frustum-Culling)
    // #[inline]
    // pub fn contains_aabb(&self, aabb: &Aabb, world_from_local: &Affine3A) -> bool {
    //     for half_space in &self.half_spaces {
    //         if !aabb.is_in_half_space(half_space, world_from_local) {
    //             return false;
    //         }
    //     }
    //     true
    // }

    // /// Optimized version of [`Self::contains_aabb`] when the AABB is already in world space.
    // /// Use this when `world_from_local` would be [`Affine3A::IDENTITY`].
    // #[inline]
    // pub fn contains_aabb_identity(&self, aabb: &Aabb) -> bool {
    //     for half_space in &self.half_spaces {
    //         if !aabb.is_in_half_space_identity(half_space) {
    //             return false;
    //         }
    //     }
    //     true
    // }
}

pub struct CubeMapFace {
    pub target: Vec3,
    pub up: Vec3,
}

// Cubemap faces are [+X, -X, +Y, -Y, +Z, -Z], per https://www.w3.org/TR/webgpu/#texture-view-creation
// Note: Cubemap coordinates are left-handed y-up, unlike the rest of Bevy.
// See https://registry.khronos.org/vulkan/specs/1.2/html/chap16.html#_cube_map_face_selection
//
// For each cubemap face, we take care to specify the appropriate target/up axis such that the rendered
// texture using Bevy's right-handed y-up coordinate space matches the expected cubemap face in
// left-handed y-up cubemap coordinates.
pub const CUBE_MAP_FACES: [CubeMapFace; 6] = [
    // +X
    CubeMapFace {
        target: Vec3::X,
        up: Vec3::Y,
    },
    // -X
    CubeMapFace {
        target: Vec3::NEG_X,
        up: Vec3::Y,
    },
    // +Y
    CubeMapFace {
        target: Vec3::Y,
        up: Vec3::Z,
    },
    // -Y
    CubeMapFace {
        target: Vec3::NEG_Y,
        up: Vec3::NEG_Z,
    },
    // +Z (with left-handed conventions, pointing forwards)
    CubeMapFace {
        target: Vec3::NEG_Z,
        up: Vec3::Y,
    },
    // -Z (with left-handed conventions, pointing backwards)
    CubeMapFace {
        target: Vec3::Z,
        up: Vec3::Y,
    },
];

pub fn face_index_to_name(face_index: usize) -> &'static str {
    match face_index {
        0 => "+x",
        1 => "-x",
        2 => "+y",
        3 => "-y",
        4 => "+z",
        5 => "-z",
        _ => "invalid",
    }
}

#[derive(Clone, Debug, Default)]
pub struct CubemapFrusta {
    pub frusta: [Frustum; 6],
}

impl CubemapFrusta {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Frustum> {
        self.frusta.iter()
    }
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut Frustum> {
        self.frusta.iter_mut()
    }
}

/// Cubemap layout defines the order of images in a packed cubemap image.
#[derive(Default, Debug, Clone, Copy)]
pub enum CubemapLayout {
    /// layout in a vertical cross format
    /// ```text
    ///    +y
    /// -x -z +x
    ///    -y
    ///    +z
    /// ```
    #[default]
    CrossVertical = 0,
    /// layout in a horizontal cross format
    /// ```text
    ///    +y
    /// -x -z +x +z
    ///    -y
    /// ```
    CrossHorizontal = 1,
    /// layout in a vertical sequence
    /// ```text
    ///   +x
    ///   -x
    ///   +y
    ///   -y
    ///   -z
    ///   +z
    /// ```
    SequenceVertical = 2,
    /// layout in a horizontal sequence
    /// ```text
    /// +x -x +y -y -z +z
    /// ```
    SequenceHorizontal = 3,
}
