mod projection;
mod transform;

pub use projection::{CameraProjection, OrthographicProjection, PerspectiveProjection, Projection};
pub use transform::Transform;

use glam::{Mat4, Vec2, Vec3, Vec3A, Vec4, Vec4Swizzles as _};

/// A rectangle defined by two opposite corners.
///
/// The rectangle is axis aligned, and defined by its minimum and maximum coordinates,
/// stored in `Rect::min` and `Rect::max`, respectively. The minimum/maximum invariant
/// must be upheld by the user when directly assigning the fields, otherwise some methods
/// produce invalid results. It is generally recommended to use one of the constructor
/// methods instead, which will ensure this invariant is met, unless you already have
/// the minimum and maximum corners.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rect {
    /// The minimum corner point of the rect.
    pub min: Vec2,
    /// The maximum corner point of the rect.
    pub max: Vec2,
}

impl Rect {
    /// An empty `Rect`, represented by maximum and minimum corner points
    /// at `Vec2::NEG_INFINITY` and `Vec2::INFINITY`, respectively.
    /// This is so the `Rect` has a infinitely negative size.
    /// This is useful, because when taking a union B of a non-empty `Rect` A and
    /// this empty `Rect`, B will simply equal A.
    pub const EMPTY: Self = Self {
        max: Vec2::NEG_INFINITY,
        min: Vec2::INFINITY,
    };
    /// Create a new rectangle from two corner points.
    ///
    /// The two points do not need to be the minimum and/or maximum corners.
    /// They only need to be two opposite corners.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::Rect;
    /// let r = Rect::new(0., 4., 10., 6.); // w=10 h=2
    /// let r = Rect::new(2., 3., 5., -1.); // w=3 h=4
    /// ```
    #[inline]
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self::from_corners(Vec2::new(x0, y0), Vec2::new(x1, y1))
    }

    /// Create a new rectangle from two corner points.
    ///
    /// The two points do not need to be the minimum and/or maximum corners.
    /// They only need to be two opposite corners.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Rect, Vec2};
    /// // Unit rect from [0,0] to [1,1]
    /// let r = Rect::from_corners(Vec2::ZERO, Vec2::ONE); // w=1 h=1
    /// // Same; the points do not need to be ordered
    /// let r = Rect::from_corners(Vec2::ONE, Vec2::ZERO); // w=1 h=1
    /// ```
    #[inline]
    pub fn from_corners(p0: Vec2, p1: Vec2) -> Self {
        Self {
            min: p0.min(p1),
            max: p0.max(p1),
        }
    }

    /// Create a new rectangle from its center and size.
    ///
    /// # Panics
    ///
    /// This method panics if any of the components of the size is negative.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Rect, Vec2};
    /// let r = Rect::from_center_size(Vec2::ZERO, Vec2::ONE); // w=1 h=1
    /// assert!(r.min.abs_diff_eq(Vec2::splat(-0.5), 1e-5));
    /// assert!(r.max.abs_diff_eq(Vec2::splat(0.5), 1e-5));
    /// ```
    #[inline]
    pub fn from_center_size(origin: Vec2, size: Vec2) -> Self {
        assert!(size.cmpge(Vec2::ZERO).all(), "Rect size must be positive");
        let half_size = size / 2.;
        Self::from_center_half_size(origin, half_size)
    }

    /// Create a new rectangle from its center and half-size.
    ///
    /// # Panics
    ///
    /// This method panics if any of the components of the half-size is negative.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Rect, Vec2};
    /// let r = Rect::from_center_half_size(Vec2::ZERO, Vec2::ONE); // w=2 h=2
    /// assert!(r.min.abs_diff_eq(Vec2::splat(-1.), 1e-5));
    /// assert!(r.max.abs_diff_eq(Vec2::splat(1.), 1e-5));
    /// ```
    #[inline]
    pub fn from_center_half_size(origin: Vec2, half_size: Vec2) -> Self {
        assert!(
            half_size.cmpge(Vec2::ZERO).all(),
            "Rect half_size must be positive"
        );
        Self {
            min: origin - half_size,
            max: origin + half_size,
        }
    }

    /// Check if the rectangle is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Rect, Vec2};
    /// let r = Rect::from_corners(Vec2::ZERO, Vec2::new(0., 1.)); // w=0 h=1
    /// assert!(r.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.min.cmpge(self.max).any()
    }

    /// Rectangle width (max.x - min.x).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::Rect;
    /// let r = Rect::new(0., 0., 5., 1.); // w=5 h=1
    /// assert!((r.width() - 5.).abs() <= 1e-5);
    /// ```
    #[inline]
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Rectangle height (max.y - min.y).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::Rect;
    /// let r = Rect::new(0., 0., 5., 1.); // w=5 h=1
    /// assert!((r.height() - 1.).abs() <= 1e-5);
    /// ```
    #[inline]
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Rectangle size.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Rect, Vec2};
    /// let r = Rect::new(0., 0., 5., 1.); // w=5 h=1
    /// assert!(r.size().abs_diff_eq(Vec2::new(5., 1.), 1e-5));
    /// ```
    #[inline]
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// Rectangle half-size.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Rect, Vec2};
    /// let r = Rect::new(0., 0., 5., 1.); // w=5 h=1
    /// assert!(r.half_size().abs_diff_eq(Vec2::new(2.5, 0.5), 1e-5));
    /// ```
    #[inline]
    pub fn half_size(&self) -> Vec2 {
        self.size() * 0.5
    }

    /// The center point of the rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Rect, Vec2};
    /// let r = Rect::new(0., 0., 5., 1.); // w=5 h=1
    /// assert!(r.center().abs_diff_eq(Vec2::new(2.5, 0.5), 1e-5));
    /// ```
    #[inline]
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// Check if a point lies within this rectangle, inclusive of its edges.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::Rect;
    /// let r = Rect::new(0., 0., 5., 1.); // w=5 h=1
    /// assert!(r.contains(r.center()));
    /// assert!(r.contains(r.min));
    /// assert!(r.contains(r.max));
    /// ```
    #[inline]
    pub fn contains(&self, point: Vec2) -> bool {
        (point.cmpge(self.min) & point.cmple(self.max)).all()
    }

    /// Build a new rectangle formed of the union of this rectangle and another rectangle.
    ///
    /// The union is the smallest rectangle enclosing both rectangles.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Rect, Vec2};
    /// let r1 = Rect::new(0., 0., 5., 1.); // w=5 h=1
    /// let r2 = Rect::new(1., -1., 3., 3.); // w=2 h=4
    /// let r = r1.union(r2);
    /// assert!(r.min.abs_diff_eq(Vec2::new(0., -1.), 1e-5));
    /// assert!(r.max.abs_diff_eq(Vec2::new(5., 3.), 1e-5));
    /// ```
    #[inline]
    pub fn union(&self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Build a new rectangle formed of the union of this rectangle and a point.
    ///
    /// The union is the smallest rectangle enclosing both the rectangle and the point. If the
    /// point is already inside the rectangle, this method returns a copy of the rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Rect, Vec2};
    /// let r = Rect::new(0., 0., 5., 1.); // w=5 h=1
    /// let u = r.union_point(Vec2::new(3., 6.));
    /// assert!(u.min.abs_diff_eq(Vec2::ZERO, 1e-5));
    /// assert!(u.max.abs_diff_eq(Vec2::new(5., 6.), 1e-5));
    /// ```
    #[inline]
    pub fn union_point(&self, other: Vec2) -> Self {
        Self {
            min: self.min.min(other),
            max: self.max.max(other),
        }
    }

    /// Build a new rectangle formed of the intersection of this rectangle and another rectangle.
    ///
    /// The intersection is the largest rectangle enclosed in both rectangles. If the intersection
    /// is empty, this method returns an empty rectangle ([`Rect::is_empty()`] returns `true`), but
    /// the actual values of [`Rect::min`] and [`Rect::max`] are implementation-dependent.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Rect, Vec2};
    /// let r1 = Rect::new(0., 0., 5., 1.); // w=5 h=1
    /// let r2 = Rect::new(1., -1., 3., 3.); // w=2 h=4
    /// let r = r1.intersect(r2);
    /// assert!(r.min.abs_diff_eq(Vec2::new(1., 0.), 1e-5));
    /// assert!(r.max.abs_diff_eq(Vec2::new(3., 1.), 1e-5));
    /// ```
    #[inline]
    pub fn intersect(&self, other: Self) -> Self {
        let mut r = Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        };
        // Collapse min over max to enforce invariants and ensure e.g. width() or
        // height() never return a negative value.
        r.min = r.min.min(r.max);
        r
    }

    /// Create a new rectangle by expanding it evenly on all sides.
    ///
    /// A positive expansion value produces a larger rectangle,
    /// while a negative expansion value produces a smaller rectangle.
    /// If this would result in zero or negative width or height, [`Rect::EMPTY`] is returned instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Rect, Vec2};
    /// let r = Rect::new(0., 0., 5., 1.); // w=5 h=1
    /// let r2 = r.inflate(3.); // w=11 h=7
    /// assert!(r2.min.abs_diff_eq(Vec2::splat(-3.), 1e-5));
    /// assert!(r2.max.abs_diff_eq(Vec2::new(8., 4.), 1e-5));
    ///
    /// let r = Rect::new(0., -1., 6., 7.); // w=6 h=8
    /// let r2 = r.inflate(-2.); // w=11 h=7
    /// assert!(r2.min.abs_diff_eq(Vec2::new(2., 1.), 1e-5));
    /// assert!(r2.max.abs_diff_eq(Vec2::new(4., 5.), 1e-5));
    /// ```
    #[inline]
    pub fn inflate(&self, expansion: f32) -> Self {
        let mut r = Self {
            min: self.min - expansion,
            max: self.max + expansion,
        };
        // Collapse min over max to enforce invariants and ensure e.g. width() or
        // height() never return a negative value.
        r.min = r.min.min(r.max);
        r
    }

    /// Build a new rectangle from this one with its coordinates expressed
    /// relative to `other` in a normalized ([0..1] x [0..1]) coordinate system.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Rect, Vec2};
    /// let r = Rect::new(2., 3., 4., 6.);
    /// let s = Rect::new(0., 0., 10., 10.);
    /// let n = r.normalize(s);
    ///
    /// assert_eq!(n.min.x, 0.2);
    /// assert_eq!(n.min.y, 0.3);
    /// assert_eq!(n.max.x, 0.4);
    /// assert_eq!(n.max.y, 0.6);
    /// ```
    pub fn normalize(&self, other: Self) -> Self {
        let outer_size = other.size();
        Self {
            min: (self.min - other.min) / outer_size,
            max: (self.max - other.min) / outer_size,
        }
    }

    /// Return the area of this rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::Rect;
    /// let r = Rect::new(0., 0., 10., 10.); // w=10 h=10
    /// assert_eq!(r.area(), 100.0);
    /// ```
    #[inline]
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }
}

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
