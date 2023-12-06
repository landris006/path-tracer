struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
};

@group(0) @binding(1) var outputTex: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_ix: vec3<u32>) {
    let screen_size: vec2<f32> = vec2<f32>(textureDimensions(outputTex));

    let fragCoord: vec2<f32> = vec2<f32>(global_ix.xy) / screen_size;

    let fragColor: vec4<f32> = vec4<f32>(fragCoord.x, fragCoord.y, 0.5, 1.0);

    textureStore(outputTex, vec2<i32>(global_ix.xy), fragColor);
}
