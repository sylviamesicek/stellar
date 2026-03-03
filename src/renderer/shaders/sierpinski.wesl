import super::fullscreen::{fullscreen, VertexOutput};

const maximum_ray_steps: u32 = 100;
const maximum_distance: f32 = 1000.0;
const minimum_distance: f32 = 0.01;
const pi: f32 = 3.141592653589793238;


// ********************
// SDF functions

// Sphere
fn signed_dist_sphere(p: vec3f, radius: f32) -> f32 {
    return length(p) - radius;
}
// Box
fn signed_dist_box(p: vec3f, size: f32) -> f32 {
    let d = abs(p) - size;
    return min(max(d.x, max(d.y, d.z)), 0.0) + length(max(d, vec3f(0.0)));
}
// Infinite Plane
// n.xyz: normal of the plane
// n.w: offset from origin
fn signed_dist_plane(p: vec3f, n: vec4<f32>) -> f32 {
    return dot(p, n.xyz) + n.w;
}
// Rounded box
// r: radius of rounded edges
fn signed_dist_round_box(p: vec3f, sides: vec3f, r: f32) -> f32 {
    let q = abs(p) - sides;
    return min(max(q.x, max(q.y, q.z)), 0.0) + length(max(q, vec3f(0.0))) - r;
}

// ***********************
// Boolean operators

// Union
// d1: signed distance to shape 1
// d2: signed distance to shape 2

// ***********************
// Main

fn hsv2rgb(color: vec3f) -> vec3f {
    const K: vec4f = vec4f(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p: vec3f = abs(fract(color.xxx + K.xyz) * 6.0 - K.www);
    return color.z * mix(K.xxx, clamp(p - K.xxx, vec3f(0.0), vec3f(1.0)), color.y);
}

fn sierpinski3(p: vec3f) -> f32 {
    var z: vec3f = p;

    let iterations = 15;
    let scale = 2.0;
    let offset = 3.0;
    
    var n = 0;

    while n < iterations {
        // Fold 1
        if z.x + z.y < 0.0 {
            z = vec3(-z.y, -z.x, z.z);
        }
        // Fold 2
        if z.x + z.z < 0.0 {
            z = vec3(-z.z, z.y, -z.x);
        }
        // Fold 3
        if z.y + z.z < 0.0 {
            z = vec3(z.x, -z.z, -z.y);
        }
        z = z * scale - offset * (scale - 1.0);
        n += 1;
    }
    return length(z) * pow(scale, -f32(n));
}

fn rotate(angle: f32) -> mat2x2f {
    let s = sin(angle);
    let c = cos(angle);
    return mat2x2(c, -s, s, c);
}

fn distance_estimator(point: vec3f) -> f32 {
    var p = point;
    let pyz = rotate(0.20 * pi) * p.yz;
    p = vec3(p.x, pyz);
    let pyx = rotate(0.25 * pi) * p.yx;
    p = vec3(pyx.yx, p.z);
    return sierpinski3(p);
}

fn ray_marcher(ro: vec3f, rd: vec3f) -> vec4f {
    var steps: u32 = 0;
    var total_distance: f32 = 0.0;
    var min_dist_to_scene: f32 = 100.0;
    var min_dist_to_sceen_pos: vec3f = ro;
    var min_dist_to_origin: f32 = 100.0;
    var min_dist_to_origin_pos: vec3f = ro;
    var color: vec3f = vec3(0.0, 0.0, 0.0);
    var cur_pos: vec3f = ro;
    var hit: bool = false;

    for (steps = 0; steps < maximum_ray_steps; steps++) {
        let p = ro + total_distance * rd; // Current position of ray
        let dist = distance_estimator(p);
        cur_pos = ro + rd * total_distance;
        if min_dist_to_scene > dist {
            min_dist_to_scene = dist;
            min_dist_to_origin_pos = cur_pos;
        }
        if min_dist_to_origin > length(cur_pos) {
            min_dist_to_origin = length(cur_pos);
            min_dist_to_origin_pos = cur_pos;
        }
        total_distance += dist;
        if dist < minimum_distance {
            hit = true;
            break;
        } else if dist > maximum_distance {
            break;
        }
    }

    let iterations = f32(steps) + log(log(maximum_distance)) / log(2.0) - log(log(dot(cur_pos, cur_pos))) / log(2.0);

    if hit {
        color = vec3(0.8 + length(cur_pos) / 8.5, 1.0, 0.8);
        color = hsv2rgb(color);
    } else {
        // color = vec3(0.8 + length(min_dist_to_sceen_pos) / 8.0, 1.0, 0.8);
        // color = hsv2rgb(color);
        // color /= min_dist_to_scene;
        color = vec3(0.0);
    }

    color /= iterations / 10.0;
    // color /= pow(distance(ro, min_dist_to_origin_pos), 2.0);
    // color *= 2000.0;

    return vec4(color, 1.0);
}

fn create_ray(px: vec2f, pinv: mat4x4f, vinv: mat4x4f) -> vec3f {
    let camera_pos = vinv[3].xyz;

    // Transform to ndc while switching x axis
    let ndc = (px * 2.0 - vec2(1.0)) * vec2(1.0, -1.0);
    var far_plane = vinv * pinv * vec4(ndc, 1.0, 1.0);
    far_plane /= far_plane.w; // Perspective divide
    return normalize(far_plane.xyz - camera_pos);


    // // Convert pixel to nds
    // // [0, 1] -> [-1, 1] (x axis)
    // // [0, 1] -> [1, -1] (y axis)
    // let px_nds = vec2(px.x * 2.0 - 1.0, 1.0 - px.y * 2.0);

    // // Choose an arbitrary point in the viewing volume
    // // z = 0.0 equals a point on the near plane
    // let point_nds = vec3(px_nds, 0.0);
    // let point_ndsh = vec4(point_nds, 1.0);
    // var dir_eye: vec4f = pinv * point_ndsh;
    // dir_eye.w = 0.0;

    // let dir_world = (vinv * dir_eye).xyz;

    // return normalize(dir_world);
}

struct CameraUniform {
    proj: mat4x4f,
    view: mat4x4f,
    inv_proj: mat4x4f,
    inv_view: mat4x4f,
}

@group(0) 
@binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    return fullscreen(vertex_index);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_coords: vec2f = in.tex_coords;

    let ray_dir = create_ray(tex_coords, camera.inv_proj, camera.inv_view);
    let ray_origin = camera.inv_view[3].xyz;
    let color = ray_marcher(ray_origin, ray_dir);
    return vec4(color.xyz, 1.0);

    // return vec4(tex_coords, 0.0, 1.0);
    // return vec4((ray_dir.xy + vec2(1.0)) / 2.0, 0.0, 1.0);
}