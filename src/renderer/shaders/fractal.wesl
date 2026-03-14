const maximum_ray_steps: u32 = 250;
const maximum_distance: f32 = 1000.0;
const minimum_distance: f32 = 0.0001;
const pi: f32 = 3.141592653589793238;

// **********************
// Bindings *************

override fractal_type: u32 = 0;

struct CameraUniform {
    proj: mat4x4f,
    view: mat4x4f,
    inv_proj: mat4x4f,
    inv_view: mat4x4f,
}

struct GlobalUniform {
    time: f32,
    pre_saturation: f32,
    post_saturation: f32,
    gamma: f32,
    exposure: f32,
}

@group(0) 
@binding(0)
var<uniform> camera: CameraUniform;

@group(0)
@binding(1)
var<uniform> global: GlobalUniform;

fn hsv2rgb(color: vec3f) -> vec3f {
    const K: vec4f = vec4f(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p: vec3f = abs(fract(color.xxx + K.xyz) * 6.0 - K.www);
    return color.z * mix(K.xxx, clamp(p - K.xxx, vec3f(0.0), vec3f(1.0)), color.y);
}

fn map(value: f32, min1: f32, max1: f32, min2: f32, max2: f32) -> f32 {
    return min2 + (value - min1) * (max2 - min2) / (max1 - min1);
}

fn rotate(angle: f32) -> mat2x2f {
    let s = sin(angle);
    let c = cos(angle);
    return mat2x2(c, -s, s, c);
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

fn mandelbulb(position: vec3f) -> f32 {
    var z = position;
    var dr = 1.0;
    var r = 0.0;
    var iterations = 0;
    let power = 8.0 + (5.0 * map(sin(global.time * pi / 5.0 + pi), -1.0, 1.0, 0.0, 1.0));

    for (var i = 0; i < 10; i++) {
        iterations = i;
        r = length(z);

        if r > 2.0 {
            break;
        }

        // convert to polar coordinates
        var theta = acos(z.z / r);
        var phi = atan2(z.y, z.x);
        dr = pow(r, power - 1.0) * power * dr + 1.0;

        // scale and rotate the point
        let zr = pow(r, power);
        theta = theta * power;
        phi = phi * power;

        // convert back to cartesian coordinates
        z = zr * vec3(sin(theta) * cos(phi), sin(phi) * sin(theta), cos(theta));
        z += position;
    }
    return 0.5 * log(r) * r / dr;
}

fn distance_estimator(point: vec3f) -> f32 {
    var p = point;

    if fractal_type == 1 {
        // Sierpinski's tetrahedra
        let pyz = rotate(0.20 * pi) * p.yz;
        p = vec3(p.x, pyz);
        let pyx = rotate(0.25 * pi) * p.yx;
        p = vec3(pyx.yx, p.z);
        return sierpinski3(p);
    } else if fractal_type == 0 {
        // Mandlebulb
        let pyz = rotate(-0.3 * pi) * p.yz;
        p = vec3(p.x, pyz);
        return mandelbulb(p);
    } else {
        return maximum_distance + 1.0;
    }
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

    // if hit && fractal_type == 1 {
    //     color = vec3(0.8 + length(cur_pos) / 8.5, 1.0, 0.8);
    //     color = hsv2rgb(color);
    // } else if hit && fractal_type == 0 {
    //     color = vec3(0.8 + length(cur_pos) / 0.5, 1.0, 0.8);
    //     color = hsv2rgb(color);
    // } else if fractal_type == 0 {
    //     color = vec3(0.8 + length(min_dist_to_sceen_pos) / 0.5, 1.0, 0.8);
    //     color = hsv2rgb(color);
    //     color *= 1.0 / (min_dist_to_scene * min_dist_to_scene);
    //     color /= map(sin(global.time * 3.0), -1.0, 1.0, 3000.0, 50000.0);
    // }

    if fractal_type == 1 {
        if hit {
            color = vec3(0.8 + length(cur_pos) / 8.5, 1.0, 0.8);
            color = hsv2rgb(color);
        } else {
            // color = vec3(0.8 + length(min_dist_to_sceen_pos) / 8.0, 1.0, 0.8);
            // color = hsv2rgb(color);
            // color /= min_dist_to_scene;
            color = vec3(0.0);

            // color = vec3(0.8 + length(min_dist_to_sceen_pos) / 0.5, 1.0, 0.8);
            // color = hsv2rgb(color);
            // color *= 1.0 / (min_dist_to_scene * min_dist_to_scene);
            // color /= map(sin(global.time * 3.0), -1.0, 1.0, 3000.0, 50000.0);
        }

        color /= f32(steps) * 0.08;

        // color /= iterations / 10.0;
        // color /= pow(distance(ro, min_dist_to_origin_pos), 2.0);
        // color *= 2000.0;
    } else if fractal_type == 0 {
        if hit {
            color = vec3(0.8 + length(cur_pos) / 0.5, 1.0, 0.8);
            color = hsv2rgb(color);
        } else {
            color = vec3(0.8 + length(min_dist_to_sceen_pos) / 0.5, 1.0, 0.8);
            color = hsv2rgb(color);
            color *= 1.0 / (min_dist_to_scene * min_dist_to_scene);
            color /= map(sin(global.time * 3.0), -1.0, 1.0, 3000.0, 50000.0);
        }

        color /= f32(steps) * 0.08; // Ambient occlusion
        // color /= pow(distance(ro, min_dist_to_sceen_pos), 2.0);
        // color *= 3.0;
    }

    return vec4(color, 1.0);
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
fn fs_main(@location(0) uv: vec2f) -> @location(0) vec4<f32> {
    let ray_dir = create_ray(uv, camera.inv_proj, camera.inv_view);
    let ray_origin = camera.inv_view[3].xyz;
    let color = ray_marcher(ray_origin, ray_dir);
    return vec4(color.xyz, 1.0);
}