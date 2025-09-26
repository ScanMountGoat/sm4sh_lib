@group(0) @binding(0)
var color1: texture_2d<f32>;

@group(0) @binding(1)
var color2: texture_2d<f32>;

@group(0) @binding(2)
var color3: texture_2d<f32>;

@group(0) @binding(3)
var color4: texture_2d<f32>;

@group(0) @binding(4)
var color_sampler: sampler;

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
    // TODO: port the logic from in game shader
    // TODO: Figure out the actual shader used from the nsh?
    let color1 = textureSample(color1, color_sampler, in.uv);
    let color2 = textureSample(color2, color_sampler, in.uv);
    let color3 = textureSample(color3, color_sampler, in.uv);
    let color4 = textureSample(color4, color_sampler, in.uv);
    return (color1 + color2 + color3 + color4) / 4.0;
}