const PI: f32 = 3.1415926535897932384626433832795;

fn rand(seed: vec2<f32>) -> f32 {
    return fract(sin(dot(seed, vec2(12.9898, 78.233))) * 43758.5453);
}

fn rand11(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453123);
}

fn rand_min_max(seed: vec2<f32>, min: f32, max: f32) -> f32 {
    return min + (max - min) * rand(seed);
}

fn randomUnit(seed: vec2<f32>) -> vec3<f32> {
    let z = rand(seed) * 2.0 - 1.0;
    let a = rand(seed * z) * 2.0 * PI;
    let r = sqrt(1.0 - z * z);
    let x = r * cos(a);
    let y = r * sin(a);
    return vec3<f32>(x, y, z);
}

fn randomUnitInHemisphere(seed: vec2<f32>, normal: vec3<f32>) -> vec3<f32> {
    let inUnitSphere = randomUnit(seed);
    if dot(inUnitSphere, normal) > 0.0 {
        return inUnitSphere;
    } else {
        return -inUnitSphere;
    }
}

fn tauStep(z: u32, s1: i32, s2: i32, s3: i32, m: u32) -> u32 {
    let b = ((z << u32(s1)) ^ z) >> u32(s2);
    return ((z & m) << u32(s3)) ^ b;
}

fn lcgStep(z: u32, a: u32, c: u32) -> u32 {
    return (a * z + c);
}

struct RandomResult {
  state: vec4<u32>,
  value: f32,
}

fn hybridTaus(state: ptr<function, vec4<u32>>) -> RandomResult {
    (*state).x = tauStep((*state).x, 13, 19, 12, 4294967294u);
    (*state).y = tauStep((*state).y, 2, 25, 4, 4294967288u);
    (*state).z = tauStep((*state).z, 3, 11, 17, 4294967280u);
    (*state).w = lcgStep((*state).w, 1664525u, 1013904223u);

    var randomResult = RandomResult();
    randomResult.state = *state;
    randomResult.value = 2.3283064365387e-10 * f32((*state).x ^ (*state).y ^ (*state).z ^ (*state).w);

    return randomResult;
}
