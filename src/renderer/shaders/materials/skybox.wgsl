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

struct StarDesc {
    origin: vec3f,
    radius: f32,
    color: vec3f,
    sunspot_threshold: f32,
    sunspot_frequency: f32,
    granule_frequency: f32,
    grandule_persistence: f32,
    time_scale: f32,
}

@group(1) @binding(0) var instances: texture_2d<u32>;
@group(1) @binding(1) var positions: texture_2d<f32>;
@group(1) @binding(2) var directions: texture_2d<f32>;

@group(2) @binding(0) var panorama: texture_2d<f32>;
@group(2) @binding(1) var panorama_sampler: sampler;

@fragment
fn fs_main(@location(0) uv: vec2f) -> @location(0) vec4f {
    let size = textureDimensions(instances);
    let coord = vec2u(vec2f(size) * uv);

    let direction = normalize(textureLoad(directions, coord, 0).xyz);

    let theta = atan2(direction.z, direction.x);
    let u = theta / (2.0 * pi) + 0.5;

    let phi = asin(direction.y);
    let v = phi / pi + 0.5;

    let color = textureSample(panorama, panorama_sampler, vec2(u, v));

    return vec4(color.rgb, 1.0);
}