@group(0) @binding(0)
var depth: texture_depth_2d;

@group(0) @binding(2)
var depth_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    // A fullscreen triangle using index calculations.
    var out: VertexOutput;
    let x = f32((i32(in_vertex_index) << 1u) & 2);
    let y = f32(i32(in_vertex_index & 2u));
    out.position = vec4(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2(x, 1.0 - y);
    return out;
}

// TODO: Should this use compute instead?
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // TODO: Figure out the actual shader used from the nsh?
    let dimensions = textureDimensions(depth);
    let offset = 1.0 / vec2<f32>(dimensions);
    let depth1 = textureSample(depth, depth_sampler, in.uv + vec2(offset.x, offset.y));
    let depth2 = textureSample(depth, depth_sampler, in.uv + vec2(-offset.x, -offset.y));
    let depth3 = textureSample(depth, depth_sampler, in.uv + vec2(offset.x, -offset.y));
    let depth4 = textureSample(depth, depth_sampler, in.uv + vec2(-offset.x, -offset.y));

    // Calculate an approximation of the first two moments M1 and M2.
    // M1 is the mean, and M2 is the square of M1.
    // This enables calculating smooth variance shadows in the model shader.
    let m1 = (depth1 + depth2 + depth3 + depth4) / 4.0;
    let m2 = m1 * m1;
    return vec4(m1, m2, 0.0, 0.0);
}