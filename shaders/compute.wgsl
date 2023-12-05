struct Config {
    width: u32,
    height: u32,
};

@group(0) @binding(0) var<uniform> config: Config;
@group(0) @binding(1) var outputTex: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_ix: vec3<u32>) {
    let fragCoord: vec2<f32> = vec2<f32>(global_ix.xy) / vec2<f32>(f32(config.width), f32(config.height)) - vec2<f32>(0.5, 0.5);

    let fragColor: vec4<f32> = vec4<f32>(fragCoord.x + 0.5, fragCoord.y + 0.5, 0.5, 1.0);

    textureStore(outputTex, vec2<i32>(global_ix.xy), fragColor);
}
