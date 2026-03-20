const maximum_ray_steps: u32 = 250;
const maximum_distance: f32 = 1000.0;
const minimum_distance: f32 = 0.0001;
const pi: f32 = 3.141592653589793238;

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

struct StarUniform {
    origin: vec3f,
    radius: f32,
    color: vec3f,
    sunspot_threshold: f32,
    sunspot_frequency: f32,
    granule_frequency: f32,
    grandule_persistence: f32,
    time_scale: f32,
}

@group(1) @binding(0)
var<uniform> star: StarUniform;

//  MIT License. © Ian McEwan, Stefan Gustavson, Munrocket
//
fn permute_four(x: vec4<f32>) -> vec4<f32> { return ((x * 34. + 1.) * x) % vec4<f32>(289.); }
fn taylor_inv_sqrt_four(r: vec4<f32>) -> vec4<f32> { return 1.79284291400159 - 0.85373472095314 * r; }

fn simplex_noise_3d(v: vec3<f32>) -> f32 {
    let C = vec2<f32>(1. / 6., 1. / 3.);
    let D = vec4<f32>(0., 0.5, 1., 2.);

    // First corner
    var i: vec3<f32> = floor(v + dot(v, C.yyy));
    let x0 = v - i + dot(i, C.xxx);

    // Other corners
    let g = step(x0.yzx, x0.xyz);
    let l = 1.0 - g;
    let i1 = min(g.xyz, l.zxy);
    let i2 = max(g.xyz, l.zxy);

    // x0 = x0 - 0. + 0. * C
    let x1 = x0 - i1 + 1. * C.xxx;
    let x2 = x0 - i2 + 2. * C.xxx;
    let x3 = x0 - 1. + 3. * C.xxx;

    // Permutations
    i = i % vec3<f32>(289.);
    let p = permute_four(permute_four(permute_four(
        i.z + vec4<f32>(0., i1.z, i2.z, 1.)
    ) +
      i.y + vec4<f32>(0., i1.y, i2.y, 1.)) +
      i.x + vec4<f32>(0., i1.x, i2.x, 1.));

    // Gradients (NxN points uniformly over a square, mapped onto an octahedron.)
    var n_: f32 = 1. / 7.; // N=7
    let ns = n_ * D.wyz - D.xzx;

    let j = p - 49. * floor(p * ns.z * ns.z); // mod(p, N*N)

    let x_ = floor(j * ns.z);
    let y_ = floor(j - 7.0 * x_); // mod(j, N)

    let x = x_ * ns.x + ns.yyyy;
    let y = y_ * ns.x + ns.yyyy;
    let h = 1.0 - abs(x) - abs(y);

    let b0 = vec4<f32>(x.xy, y.xy);
    let b1 = vec4<f32>(x.zw, y.zw);

    let s0 = floor(b0) * 2.0 + 1.0;
    let s1 = floor(b1) * 2.0 + 1.0;
    let sh = -step(h, vec4<f32>(0.));

    let a0 = b0.xzyw + s0.xzyw * sh.xxyy;
    let a1 = b1.xzyw + s1.xzyw * sh.zzww;

    var p0: vec3<f32> = vec3<f32>(a0.xy, h.x);
    var p1: vec3<f32> = vec3<f32>(a0.zw, h.y);
    var p2: vec3<f32> = vec3<f32>(a1.xy, h.z);
    var p3: vec3<f32> = vec3<f32>(a1.zw, h.w);

    // Normalise gradients
    let norm = taylor_inv_sqrt_four(vec4<f32>(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3)));
    p0 = p0 * norm.x;
    p1 = p1 * norm.y;
    p2 = p2 * norm.z;
    p3 = p3 * norm.w;

    // Mix final noise value
    var m: vec4<f32> = 0.6 - vec4<f32>(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3));
    m = max(m, vec4<f32>(0.));
    m = m * m;
    return 42. * dot(m * m, vec4<f32>(dot(p0, x0), dot(p1, x1), dot(p2, x2), dot(p3, x3)));
}

fn mod289(x: vec4f) -> vec4f {
    return x - floor(x * (1. / 289.)) * 289.;
}

//! requires mod289 taylorInvSqrt4
fn snoise_permute4(x: vec4<f32>) -> vec4<f32> {
    return mod289(((x * 34.0) + 10.0) * x);
}

fn snoise_mod289f(x: f32) -> f32 {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn snoise_permute(x: f32) -> f32 {
    return snoise_mod289f(((x * 34.0) + 10.0) * x);
}

fn snoise_taylorInvSqrt(r: f32) -> f32 {
    return 1.79284291400159 - 0.85373472095314 * r;
}

fn snoise_grad4(j: f32, ip: vec4<f32>) -> vec4<f32> {
    let ones = vec4(1.0, 1.0, 1.0, -1.0);

    var p: vec4<f32>;
    var s: vec4<f32>;

    p = vec4(floor(fract(vec3(j) * ip.xyz) * 7.0) * ip.z - 1.0, p.w);
    p.w = 1.5 - dot(abs(p.xyz), ones.xyz);
    s = select(vec4(0.0), vec4(1.0), p < vec4(0.0));
    p = vec4(p.xyz + (s.xyz * 2.0 - 1.0) * s.www, p.w);

    return p;
}

fn simplex_noise_4d(v: vec4<f32>) -> f32 {
    let C: vec4<f32> = vec4(
        0.138196601125011, // (5 - sqrt(5))/20  G4
        0.276393202250021, // 2 * G4
        0.414589803375032, // 3 * G4
        -0.447213595499958// -1 + 4 * G4
    );

    // (sqrt(5) - 1)/4 = F4, used once below
    let F4: f32 = 0.309016994374947451;

    // First corner
    var i: vec4<f32> = floor(v + dot(v, vec4(F4)));
    var x0: vec4<f32> = v - i + dot(i, C.xxxx);

    // Other corners

    // Rank sorting originally contributed by Bill Licea-Kane, AMD (formerly ATI)
    var i0: vec4<f32>;
    var isX: vec3<f32> = step(x0.yzw, x0.xxx);
    var isYZ: vec3<f32> = step(x0.zww, x0.yyz);

    //  i0.x = dot( isX, vec3( 1.0 ) );
    i0.x = isX.x + isX.y + isX.z;
    var minusX = 1.0 - isX;
    i0.y = minusX.x;
    i0.z = minusX.y;
    i0.w = minusX.z;

    //  i0.y += dot( isYZ.xy, vec2( 1.0 ) );
    i0.y += isYZ.x + isYZ.y;
    var minusY = 1.0 - isYZ;
    i0.z += minusY.x;
    i0.w += minusY.y;
    i0.z += isYZ.z;
    i0.w += minusY.z;

    // i0 now contains the unique values 0,1,2,3 in each channel
    var i3: vec4<f32> = vec4<f32>(clamp(i0, vec4(0.0), vec4(1.0)));
    var i2: vec4<f32> = clamp(i0 - vec4(1.0), vec4(0.0), vec4(1.0));
    var i1: vec4<f32> = clamp(i0 - vec4(2.0), vec4(0.0), vec4(1.0));

    //  x0 = x0 - 0.0 + 0.0 * C.xxxx
    //  x1 = x0 - i1  + 1.0 * C.xxxx
    //  x2 = x0 - i2  + 2.0 * C.xxxx
    //  x3 = x0 - i3  + 3.0 * C.xxxx
    //  x4 = x0 - 1.0 + 4.0 * C.xxxx
    var x1: vec4<f32> = x0 - i1 + C.xxxx;
    var x2: vec4<f32> = x0 - i2 + C.yyyy;
    var x3: vec4<f32> = x0 - i3 + C.zzzz;
    var x4: vec4<f32> = x0 + C.wwww;

    // Permutations
    i = mod289(i);
    var j0: f32 = snoise_permute(snoise_permute(snoise_permute(snoise_permute(i.w) + i.z) + i.y) + i.x);
    var j1: vec4<f32> = snoise_permute4(snoise_permute4(snoise_permute4(snoise_permute4(i.w + vec4(i1.w, i2.w, i3.w, 1.0)) + i.z + vec4(i1.z, i2.z, i3.z, 1.0)) + i.y + vec4(i1.y, i2.y, i3.y, 1.0)) + i.x + vec4(i1.x, i2.x, i3.x, 1.0));

    // Gradients: 7x7x6 points over a cube, mapped onto a 4-cross polytope
    // 7*7*6 = 294, which is close to the ring size 17*17 = 289.
    var ip: vec4<f32> = vec4(1.0 / 294.0, 1.0 / 49.0, 1.0 / 7.0, 0.0);

    var p0: vec4<f32> = snoise_grad4(j0, ip);
    var p1: vec4<f32> = snoise_grad4(j1.x, ip);
    var p2: vec4<f32> = snoise_grad4(j1.y, ip);
    var p3: vec4<f32> = snoise_grad4(j1.z, ip);
    var p4: vec4<f32> = snoise_grad4(j1.w, ip);

    // Normalise gradients
    var norm: vec4<f32> = taylor_inv_sqrt_four(vec4(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3)));
    p0 *= norm.x;
    p1 *= norm.y;
    p2 *= norm.z;
    p3 *= norm.w;
    p4 *= snoise_taylorInvSqrt(dot(p4, p4));

    // Mix contributions from the five corners
    var m0: vec3<f32> = max(0.6 - vec3(dot(x0, x0), dot(x1, x1), dot(x2, x2)), vec3(0.0));
    var m1: vec2<f32> = max(0.6 - vec2(dot(x3, x3), dot(x4, x4)), vec2(0.0));
    m0 = m0 * m0;
    m1 = m1 * m1;
    return 49.0 * (dot(m0 * m0, vec3(dot(p0, x0), dot(p1, x1), dot(p2, x2))) + dot(m1 * m1, vec2(dot(p3, x3), dot(p4, x4))));
}

fn noise3d(pos: vec3f, octaves: i32, frequency: f32, persistence: f32) -> f32 {
    var total = 0.0;
    var max_amplitude = 0.0;
    var amplitude = 1.0;
    var freq = frequency;

    for (var i = 0; i < octaves; i++) {
        total += simplex_noise_3d(pos * freq) * amplitude;
        freq *= 2.0;
        max_amplitude += amplitude;
        amplitude *= persistence;
    }

    return total / max_amplitude;
}

fn noise4d(pos: vec4f, octaves: i32, frequency: f32, persistence: f32) -> f32 {
    var total = 0.0;
    var max_amplitude = 0.0;
    var amplitude = 1.0;
    var freq = frequency;

    for (var i = 0; i < octaves; i++) {
        total += simplex_noise_4d(pos * freq) * amplitude;
        freq *= 2.0;
        max_amplitude += amplitude;
        amplitude *= persistence;
    }

    return total / max_amplitude;
}

fn hsv2rgb(color: vec3f) -> vec3f {
    const K: vec4f = vec4f(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p: vec3f = abs(fract(color.xxx + K.xyz) * 6.0 - K.www);
    return color.z * mix(K.xxx, clamp(p - K.xxx, vec3f(0.0), vec3f(1.0)), color.y);
}

fn distance_estimator(point: vec3f) -> f32 {
    var p = point;
    return distance(point, star.origin) - star.radius;
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

    if hit {
        // Compute the normal
        let normal = normalize(cur_pos - star.origin);
        // Normalized model position
        let npos = normal;
        // Normalized position with time
        let fpos = vec4(npos, global.time / 200.0);
        let n = (noise4d(fpos, 4, star.granule_frequency, star.grandule_persistence) + 1.0) * 0.5;
        // Scaled model position moving with time
        let spos = fpos * star.radius;
        let t1 = simplex_noise_4d(spos * star.sunspot_frequency) - star.sunspot_threshold;
        let t2 = simplex_noise_4d((spos + star.radius) * star.sunspot_frequency) - star.sunspot_threshold;
        let ss = (max(t1, 0.0) * max(t2, 0.0)) * 2.0;
        let total = n - ss;

        let theta = 1.0 - dot(npos, normalize(ro - star.origin));

        let pcolor = star.color + (total - 0.5) - theta;
        color = pcolor;
    } else {
        color = vec3(0.0);
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
fn fs_main(@location(0) uv: vec2f) -> @location(0) vec4f {
    let ray_dir = create_ray(uv, camera.inv_proj, camera.inv_view);
    let ray_origin = camera.inv_view[3].xyz;

    let color = ray_marcher(ray_origin, ray_dir);
    return vec4(color.xyz, 1.0);
}