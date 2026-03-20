const maximum_ray_steps: u32 = 250;
const maximum_distance: f32 = 1000.0;
const minimum_distance: f32 = 0.0001;
const pi: f32 = 3.141592653589793238;

const background_material_id: u32 = 255;

struct CameraUniform {
    proj: mat4x4f,
    view: mat4x4f,
    inv_proj: mat4x4f,
    inv_view: mat4x4f,
}

struct GlobalUniform {
    time: f32,
}

@group(0) 
@binding(0)
var<uniform> camera: CameraUniform;

@group(0)
@binding(1)
var<uniform> global: GlobalUniform;

struct SphereDesc {
    origin: vec3f,
    radius: f32,
    material_id: u32,
    instance_id: u32,
}

@group(1)
@binding(0)
var<storage, read> spheres: array<SphereDesc>;

struct Immediates {
    num_spheres: u32,
}
var<immediate> c: Immediates;

struct RayInfo {
    dist: f32,
    material_id: u32,
    instance_id: u32,
}

struct FragOutput {
    @location(0) instance_id: u32,
    @location(1) position: vec4f,
    @location(2) direction: vec4f,
    @builtin(frag_depth) material_id: f32,
}

fn distance_estimator(point: vec3f) -> RayInfo {
    var dist = maximum_distance + 1.0;
    var material_id: u32 = background_material_id;
    var instance_id: u32 = 0;

    for (var i: u32 = 0; i < c.num_spheres; i++) {
        let d = distance(point, spheres[i].origin) - spheres[i].radius;
        if d < dist {
            dist = d;
            material_id = spheres[i].material_id;
            instance_id = spheres[i].instance_id;
        }
    }

    return RayInfo(dist, material_id, instance_id);
}

fn ray_marcher(ro: vec3f, rd: vec3f) -> FragOutput {
    var steps: u32 = 0;
    var total_distance: f32 = 0.0;
    // var min_dist_to_scene: f32 = 100.0;
    // var min_dist_to_sceen_pos: vec3f = ro;
    // var min_dist_to_origin: f32 = 100.0;
    // var min_dist_to_origin_pos: vec3f = ro;
    var color: vec3f = vec3(0.0, 0.0, 0.0);
    var cur_pos: vec3f = ro;
    var hit: bool = false;

    for (steps = 0; steps < maximum_ray_steps; steps++) {
        let p = ro + total_distance * rd; // Current position of ray
        // Find ray information
        let info = distance_estimator(p);
        cur_pos = ro + rd * total_distance;
        // if min_dist_to_scene > info.dist {
        //     min_dist_to_scene = info.dist;
        //     min_dist_to_origin_pos = cur_pos;
        // }
        // if min_dist_to_origin > length(cur_pos) {
        //     min_dist_to_origin = length(cur_pos);
        //     min_dist_to_origin_pos = cur_pos;
        // }
        total_distance += info.dist;
        if info.dist < minimum_distance {
            return FragOutput(info.instance_id, vec4(cur_pos, info.dist), vec4(rd, 0.0), f32(info.material_id) / 256.0);
        } else if info.dist > maximum_distance {
            break;
        }
    }

    return FragOutput(0, vec4(cur_pos, 0.0), vec4(rd, 0.0), f32(background_material_id) / 256.0);
    // return FragOutput(0, vec4(cur_pos, 0.0), vec4(rd, 0.0), f32(0.0) / 256.0);

    // if hit {
    //     // Compute the normal
    //     let normal = normalize(cur_pos - star.origin);
    //     // Normalized model position
    //     let npos = normal;
    //     // Normalized position with time
    //     let fpos = vec4(npos, global.time / 200.0);
    //     let n = (noise4d(fpos, 4, star.granule_frequency, star.grandule_persistence) + 1.0) * 0.5;
    //     // Scaled model position moving with time
    //     let spos = fpos * star.radius;
    //     let t1 = simplex_noise_4d(spos * star.sunspot_frequency) - star.sunspot_threshold;
    //     let t2 = simplex_noise_4d((spos + star.radius) * star.sunspot_frequency) - star.sunspot_threshold;
    //     let ss = (max(t1, 0.0) * max(t2, 0.0)) * 2.0;
    //     let total = n - ss;

    //     let theta = 1.0 - dot(npos, normalize(ro - star.origin));

    //     let pcolor = star.color + (total - 0.5) - theta;
    //     color = pcolor;
    // } else {
    //     color = vec3(0.0);
    // }

    // return vec4(color, 1.0);
}

fn create_ray(uv: vec2f, pinv: mat4x4f, vinv: mat4x4f) -> vec3f {
    let camera_pos = vinv[3].xyz;

    // Transform to ndc while switching x axis
    let ndc = (uv * 2.0 - vec2(1.0)) * vec2(1.0, -1.0);
    var far_plane = vinv * pinv * vec4(ndc, 1.0, 1.0);
    far_plane /= far_plane.w; // Perspective divide
    return normalize(far_plane.xyz - camera_pos);
}

@fragment
fn fs_main(@location(0) uv: vec2f) -> FragOutput {
    let ray_dir = create_ray(uv, camera.inv_proj, camera.inv_view);
    let ray_origin = camera.inv_view[3].xyz;

    return ray_marcher(ray_origin, ray_dir);
}