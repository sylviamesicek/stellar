// Meant to be called with 3 vertex indices: 0, 1, 2
// draws one large triangle over the clip space like this:
// (the asterisks represent the clip space bounds)
//-1,1           1,1
// ---------------------------------
// |              *              .
// |              *           .
// |              *        .
// |              *      .
// |              *    . 
// |              * .
// |***************
// |            . 1,-1 
// |          .
// |       .
// |     .
// |   .
// |.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

fn fullscreen(vertex_index: u32) -> VertexOutput {
    // var result: VertexOutput;
    // let x = i32(index) / 2;
    // let y = i32(index) & 1;
    // let uv = vec2<f32>(
    //     f32(x) * 2.0,
    //     f32(y) * 2.0
    // );
    // result.position = vec4<f32>(
    //     uv.x * 2.0 - 1.0,
    //     1.0 - uv.y * 2.0,
    //     0.0, 1.0
    // );
    // result.uv = uv;
    // return result;

    // See the explanation above for how this works
    let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);

    return VertexOutput(clip_position, uv);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    return fullscreen(vertex_index);
}
