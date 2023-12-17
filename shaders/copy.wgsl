struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    @builtin(instance_index) in_instance_index: u32
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((in_vertex_index & 1u) ^ in_instance_index);
    let y = f32((in_vertex_index >> 1u) ^ in_instance_index);
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.tex_coord = vec2<f32>(x, y);
    return out;
}


@group(0) @binding(0)
var textures: binding_array<texture_2d<f32>>;
@group(0) @binding(1)
var texture_sampler: sampler;
@group(0) @binding(2)
var<uniform> progressive_rendering_samples: u32;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0);

    for (var i = 0u; i < progressive_rendering_samples; i = i + 1u) {
        color = color + textureSample(textures[i], texture_sampler, in.tex_coord);
    }

    return color / f32(progressive_rendering_samples);
}
