const PI: f32 = 3.1415926535897932384626433832795;

fn rand(seed: vec2<f32>) -> f32 {
    return fract(sin(dot(seed * 1000.0, vec2<f32>(12.25, 10.356))) * 1054.52);
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
