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

@group(1) @binding(0) var panorama: texture_2d<f32>;
@group(1) @binding(1) var panorama_sampler: sampler;

struct Immediates {
    mass: f32,
    tolerance: f32,
    safety_factor: f32,
    min_step_size: f32,
    max_step_size: f32,
    max_adaptive_iter: i32,
    max_steps: i32,
    max_time: f32,
}
var<immediate> c: Immediates;

struct RayInfo {
    dist: f32,
    material_id: u32,
    instance_id: u32,
}

struct FragOutput {
    @location(0) color: vec4f,
}

fn create_ray(uv: vec2f, pinv: mat4x4f, vinv: mat4x4f) -> vec3f {
    let camera_pos = vinv[3].xyz;

    // Transform to ndc while switching x axis
    let ndc = (uv * 2.0 - vec2(1.0)) * vec2(1.0, -1.0);
    var far_plane = vinv * pinv * vec4(ndc, 1.0, 1.0);
    far_plane /= far_plane.w; // Perspective divide
    return normalize(far_plane.xyz - camera_pos);
}

struct System {
    r: f32,
    phi: f32,
    ur: f32,
    uphi: f32,
    arc: f32,
}

fn mul_system(s1: f32, v1: System) -> System {
    return System(
        s1 * v1.r,
        s1 * v1.phi,
        s1 * v1.ur,
        s1 * v1.uphi,
        s1 * v1.arc,
    );
}

fn fma_system(s1: f32, v1: System, v2: System) -> System {
    return System(
        s1 * v1.r + v2.r,
        s1 * v1.phi + v2.phi,
        s1 * v1.ur + v2.ur,
        s1 * v1.uphi + v2.uphi,
        s1 * v1.arc + v2.arc,
    );
}

fn adm_derivatives(t: f32, sys: System) -> System {
    let rs = 2.0 * c.mass;
    let factor = 1.0 - rs / sys.r;

    let rinv = 1.0 / sys.r;

    let u0 = sqrt(sys.ur * sys.ur + sys.uphi * sys.uphi * rinv * rinv);

    let drdt = factor * sys.ur / u0;
    let dphidt = factor * rinv * rinv * sys.uphi / u0;

    let term1 = -0.5 * u0 * rs * rinv * rinv;
    let term2 = -(sys.ur * sys.ur) / (2.0 * u0) * (rs * rinv * rinv);
    let term3 = -(sys.uphi * sys.uphi) / (2.0 * u0) * rinv * rinv * rinv * (3.0 * rs * rinv - 2.0);
    let durdt = term1 + term2 + term3;

    const duphidt = 0.0;

    let darcdt = sqrt(factor * (sys.ur * sys.ur + sys.uphi * sys.uphi * rinv * rinv));

    return System(drdt, dphidt, durdt, duphidt, darcdt);
}

struct Step {
    system: System,
    dt: f32,
}

const C = array<f32, 6>(0.0, 1.0 / 4.0, 3.0 / 8.0, 12.0 / 13.0, 1.0, 1.0 / 2.0);
const A1 = array<f32, 1>(0.25);
const A2 = array<f32, 2>(3.0 / 32.0, 9.0 / 32.0);
const A3 = array<f32, 3>(1932.0 / 2197.0, -7200.0 / 2197.0, 7296.0 / 2197.0);
const A4 = array<f32, 4>(439.0 / 216.0, -8.0, 3680.0 / 513.0, -845.0 / 4104.0);
const A5 = array<f32, 5>(-8.0 / 27.0, 2.0, -3544.0 / 2565.0, 1859.0 / 4104.0, -11.0 / 40.0);
const BU = array<f32, 6>(16.0 / 135.0, 0.0, 6656.0 / 12825.0, 28561.0 / 56430.0, -9.0 / 50.0, 2.0 / 55.0);
const BE = array<f32, 6>(25.0 / 216.0, 0.0, 1408.0 / 2565.0, 2197.0 / 4104.0, -1.0 / 5.0, 0.0);
const ORDER = 4;

fn rk45(t: f32, sys: System, t_step: f32) -> Step {
    var dt = t_step;

    for (var iter = 0; iter < c.max_adaptive_iter; iter += 1) {
        var s = sys;
        let k0 = adm_derivatives(t + C[0], s);

        s = System();
        s = fma_system(A1[0], k0, s);
        s = fma_system(dt, s, sys);
        let k1 = adm_derivatives(t + C[1] * dt, s);

        s = System();
        s = fma_system(A2[0], k0, s);
        s = fma_system(A2[1], k1, s);
        s = fma_system(dt, s, sys);
        let k2 = adm_derivatives(t + C[2] * dt, s);

        s = System();
        s = fma_system(A3[0], k0, s);
        s = fma_system(A3[1], k1, s);
        s = fma_system(A3[2], k1, s);
        s = fma_system(dt, s, sys);
        let k3 = adm_derivatives(t + C[3] * dt, s);

        s = System();
        s = fma_system(A4[0], k0, s);
        s = fma_system(A4[1], k1, s);
        s = fma_system(A4[2], k1, s);
        s = fma_system(A4[3], k1, s);
        s = fma_system(dt, s, sys);
        let k4 = adm_derivatives(t + C[4] * dt, s);

        s = System();
        s = fma_system(A4[0], k0, s);
        s = fma_system(A4[1], k1, s);
        s = fma_system(A4[2], k1, s);
        s = fma_system(A4[3], k1, s);
        s = fma_system(A5[4], k1, s);
        s = fma_system(dt, s, sys);
        let k5 = adm_derivatives(t + C[5] * dt, s);

        var error = System();
        error = fma_system(BU[0] - BE[0], k0, error);
        error = fma_system(BU[1] - BE[1], k1, error);
        error = fma_system(BU[2] - BE[2], k2, error);
        error = fma_system(BU[3] - BE[3], k3, error);
        error = fma_system(BU[4] - BE[4], k4, error);
        error = fma_system(BU[5] - BE[5], k5, error);
        error = mul_system(dt, error);

        let max_error = max(error.r, max(error.phi, max(error.ur, max(error.uphi, error.arc))));

        let factor = pow(c.tolerance / max_error, 1.0 / (f32(ORDER) + 1.0));
        let new_dt = clamp(c.safety_factor * dt * factor, c.min_step_size, c.max_step_size);

        if max_error < c.tolerance {
            var result = System();
            result = fma_system(BU[0], k0, result);
            result = fma_system(BU[1], k1, result);
            result = fma_system(BU[2], k2, result);
            result = fma_system(BU[3], k3, result);
            result = fma_system(BU[4], k4, result);
            result = fma_system(BU[5], k5, result);
            result = fma_system(dt, result, sys);

            return Step(result, new_dt);
        }

        dt = new_dt;
    }

    return Step(System(), -1.0);
}

fn skybox(dir: vec3f) -> vec4f {
    let theta = atan2(dir.z, dir.x);
    let u = theta / (2.0 * pi) + 0.5;

    let phi = asin(dir.y);
    let v = phi / pi + 0.5;

    return textureSample(panorama, panorama_sampler, vec2(u, v));
}

@fragment
fn fs_main(@location(0) uv: vec2f) -> FragOutput {
    let rs = 2.0 * c.mass;

    let ray_dir = create_ray(uv, camera.inv_proj, camera.inv_view);
    let ray_origin = camera.inv_view[3].xyz;

    // The x unit vector points from the black hole to starting
    // position of the ray
    let xunit = normalize(ray_origin);
    let ray_dir_proj = dot(ray_dir, xunit) * xunit;
    let ray_dir_orth = ray_dir - ray_dir_proj;
    // Is y_axis very small? Then there should be no angular velocity
    let is_y_parallel = dot(ray_dir_orth, ray_dir_orth) < 0.000001;
    let yunit = select(normalize(ray_dir_orth), vec3(0.0, 0.0, 0.0), is_y_parallel);

    let x = length(ray_origin);
    let y = 0.0;

    var dxdt = dot(ray_dir, xunit);
    var dydt = dot(ray_dir, yunit); // if is_y_parallel == true, this is 0.0

    let r = length(ray_origin);
    let phi = 0.0;

    let factor = 1.0 - rs / r;
    let factor_recip = 1.0 / factor;

    // Rescale d(x, y)/dt to have length (1 - rs/r). This results in u0=1 on first step.
    dxdt *= factor;
    dydt *= factor;

    let drdt = dxdt; // * cos(phi) + dydt * sin(phi);
    let dphidt = 1.0 / r * dydt; // * cos(phi) - 1/r * dxdt * sin(phi)

    let ur = factor_recip * drdt;
    let uphi = factor_recip * r * r * dphidt;

    var sys = System(r, phi, ur, uphi, 0.0);
    var time = 0.0;
    var dt = 0.01;

    for (var i = 0; i < c.max_steps && time < c.max_time && sys.r > rs * 1.01; i += 1) {
        let step = rk45(time, sys, dt);

        if step.dt < 0.0 {
            return FragOutput(vec4f(1.0, 0.0, 0.0, 1.0));
        }

        time += dt;
        sys = step.system;
        dt = step.dt;
    }

    let factor_p = (1.0 - rs / sys.r);
    let u0_p = sqrt(sys.ur * sys.ur + sys.uphi * sys.uphi / (sys.r * sys.r));

    let drdt_p = factor_p * sys.ur / u0_p;
    let dphidt_p = factor_p / (sys.r * sys.r) * sys.uphi / u0_p;

    let rv = drdt_p;
    let tv = sys.r * dphidt_p;

    let dxdt_p = rv * cos(sys.phi) - tv * sin(sys.phi);
    let dydt_p = rv * sin(sys.phi) + tv * cos(sys.phi);

    let dir = dxdt_p * xunit + dydt_p * yunit;

    if sys.r > 1.5 * rs {
        return FragOutput(skybox(dir));
    } else {
        return FragOutput(vec4f(0.0, 0.0, 0.0, 1.0));
    }
}