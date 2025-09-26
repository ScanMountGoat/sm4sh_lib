@group(0) @binding(0)
var color: texture_2d<f32>;

@group(0) @binding(2)
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
    // TODO: Figure out the actual shader used from the nsh?
    let dimensions = textureDimensions(color);
    let offset = 1.0 / vec2<f32>(dimensions);
    let color1 = textureSample(color, color_sampler, in.uv + vec2(offset.x, -offset.y));
    let color2 = textureSample(color, color_sampler, in.uv + vec2(-offset.x, -offset.y));
    let color3 = textureSample(color, color_sampler, in.uv + vec2(-offset.x, offset.y));
    let color4 = textureSample(color, color_sampler, in.uv + vec2(offset.x, offset.y));
    let average = (color1.rgb + color2.rgb + color3.rgb + color4.rgb) / 4.0;

    let component_max = max(max(average.x, 0.001), max(average.y, average.z));
    let scale = (max(component_max - 0.5, 0.0) / component_max) * 2.0;
    return vec4(average * scale, 1.0);
}