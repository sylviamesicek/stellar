//! Shader for counting number of pixels for each material ID

// Storage buffer which contains one 
@group(0) @binding(0) var<storage, read_write> material_count: array<atomic<u32>>;
@group(0) @binding(1) var material_buffer: texture_2d<u32>;

var<workgroup> local_buffer: array<u32, 256>;

fn local_compare_and_swap(idx_before: u32, idx_after: u32) {
    // idx before should always be < idx_after
    if local_buffer[idx_after] < local_buffer[idx_before] {
        var temp: u32 = local_buffer[idx_before];
        local_buffer[idx_before] = local_buffer[idx_after];
        local_buffer[idx_after] = temp;
    }
}

// invoke_id goes from 0 to workgroupSize
fn get_flip_indices(invoke_id: u32, block_height: u32) -> vec2u {
    // Caculate index offset (i.e move indices into correct block)
    let block_offset: u32 = ((2 * invoke_id) / block_height) * block_height;
    let half_height = block_height / 2;
    // Calculate index spacing
    var idx: vec2u = vec2u(
        invoke_id % half_height, block_height - (invoke_id % half_height) - 1,
    );
    idx.x += block_offset;
    idx.y += block_offset;
    return idx;
}

fn get_disperse_indices(invoke_id: u32, block_height: u32) -> vec2u {
    var block_offset: u32 = ((2 * invoke_id) / block_height) * block_height;
    let half_height = block_height / 2;
    var idx: vec2u = vec2u(
        invoke_id % half_height, (invoke_id % half_height) + half_height
    );
    idx.x += block_offset;
    idx.y += block_offset;
    return idx;
}

// Constants/enum
const ALGO_NONE = 0;
const ALGO_LOCAL_FLIP = 1;
const ALGO_LOCAL_DISPERSE = 2;
const ALGO_GLOBAL_FLIP = 3;
 
@compute @workgroup_size(128, 1, 1)
fn cs_main(
    @builtin(global_invocation_id) global_invocation_id: vec3u,
    @builtin(workgroup_id) workgroup_id: vec3u,
    @builtin(local_invocation_id) local_invocation_id: vec3u,
) {
    let size = textureDimensions(material_buffer, 0);
    // What pixel id are we on?
    let position = global_invocation_id.xy;
    // Only perform operations if we are actually on the texture
    if any(position >= size) {
        return;
    }

    // Load input data
    local_buffer[local_invocation_id.x] = textureLoad(material_buffer, position, 0).r;

    workgroupBarrier();

    // let num_materials = arrayLength(&material_count);

    // let size = textureDimensions(material_buffer, 0);
    // // What pixel id are we on?
    // let position = global_invocation_id.xy;
    // // Only perform operations if we are actually on the texture
    // if all(position < size) {
    //     // Load material id.
    //     let material_id = textureLoad(material_buffer, position, 0).r;
    //     // Compute offset material index 
    //     let packed_id = (material_id << 8) | local_invocation_id.x;
    //     // Atomically add 1 to that material count.
    //     // The true material count is actually the sum of chunk_size entries in a row
    //     atomicAdd(&material_count[packed_id], 1u);
    // }
}

var<workgroup> material_buffers_flags: array<u32, 256>;

@compute @workgroup_size(128, 1, 1)
fn cs_main_2(
    @builtin(global_invocation_id) global_invocation_id: vec3u,
    @builtin(workgroup_id) workgroup_id: vec3u,
    @builtin(local_invocation_id) local_invocation_id: vec3u,
) {
    // [0, 0, 0, 1, 0, 0, 0, 1, 0]
    var thread_id: u32 = local_invocation_id.x;
    var global_thread_id: u32 = global_invocation_id.x;

    var offset: u32 = 1;

    for (var d: u32 = 128; d > 0; d >>= 1) {
        if thread_id < d {
            var ai = offset * (2 * thread_id + 1) - 1;
        }
    }
}

