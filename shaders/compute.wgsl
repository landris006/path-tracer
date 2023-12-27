//!include "utils.wgsl"

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
};

struct Camera {
    origin: vec3<f32>,
    focalLength: f32,
    forward: vec3<f32>,
    vfov: f32,
    right: vec3<f32>,
    _padding2: f32,
    up: vec3<f32>,
    _padding3: f32,
}

struct HitRecord {
    hit: bool,
    t: f32,
    p: vec3<f32>,
    normal: vec3<f32>,
    frontFace: bool,
    attenuation: vec3<f32>,
    material: f32,
}

struct Settings {
  samplesPerPixel: u32,
  depth: u32,
  tMin: f32,
  tMax: f32,
}

struct Sphere {
  center: vec3<f32>,
  radius: f32,
  albedo: vec3<f32>,
  material: f32,
}

struct Triangle {
  v0: vec3<f32>,
  material: f32,
  v1: vec3<f32>,
  _padding1: f32,
  v2: vec3<f32>,
  _padding2: f32,
  albedo: vec3<f32>,
  _padding3: f32,
}

struct SphereData {
  sphereCount: u32,
  spheres: array<Sphere>,
}

struct TriangleData {
  triangleCount: u32,
  triangles: array<Triangle>,
}

@group(0) @binding(0) var outputTex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> sphereData: SphereData;
@group(0) @binding(3) var<uniform> time: u32;
@group(0) @binding(4) var skyTexture: texture_cube<f32>;
@group(0) @binding(5) var skyTextureSampler: sampler;
@group(0) @binding(6) var<uniform> settings: Settings;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) threadId: vec3<u32>) {
    var randomState: vec4<u32> = vec4<u32>(threadId.xy, threadId.xy + vec2<u32>(1u, 1u) * time);

    let screen_size: vec2<u32> = vec2<u32>(textureDimensions(outputTex));

    if threadId.x >= screen_size.x || threadId.y >= screen_size.y {
        return;
    }

    let aspectRatio: f32 = f32(screen_size.x) / f32(screen_size.y);

    let theta = radians(camera.vfov);
    let h = tan(theta / 2.0);
    let viewPortHeight: f32 = 2.0 * h * camera.focalLength;
    let viewPortWidth: f32 = aspectRatio * viewPortHeight;

    let viewPortU: vec3<f32> = viewPortWidth * camera.right;
    let viewPortV: vec3<f32> = -viewPortHeight * camera.up;

    let pixelDeltaU = viewPortU / f32(screen_size.x);
    let pixelDeltaV = viewPortV / f32(screen_size.y);

    let upper_left: vec3<f32> = camera.origin + camera.focalLength * camera.forward - 0.5 * (viewPortU + viewPortV);
    let pixel00Location: vec3<f32> = upper_left + 0.5 * (pixelDeltaU + pixelDeltaV);

    let pixelLocation: vec3<f32> = pixel00Location + f32(threadId.x) * pixelDeltaU + f32(threadId.y) * pixelDeltaV;

    var color: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0u; i < settings.samplesPerPixel; i = i + 1u) {
        let px: f32 = -0.5 + hybridTaus(&randomState).value;
        let py: f32 = -0.5 + hybridTaus(&randomState).value;

        let sample: vec3<f32> = pixelDeltaU * px + pixelDeltaV * py;
        let sampleLocation: vec3<f32> = pixelLocation + sample;

        let ray: Ray = Ray(camera.origin, sampleLocation - camera.origin);

        color = color + rayColor(ray, &randomState);
    }

    color = color / f32(settings.samplesPerPixel);

    let fragColor: vec4<f32> = vec4<f32>(color, 1.0);
    textureStore(outputTex, vec2<i32>(threadId.xy), fragColor);
}

fn rayColor(initialRay: Ray, randomState: ptr<function, vec4<u32>>) -> vec3<f32> {
    var color = vec3<f32>(1.0, 1.0, 1.0);
    let randomSeed = hybridTaus(randomState).value;

    var correction: u32 = 0u;

    var currentRay: Ray = initialRay;
    for (var i = 0u; i < settings.depth + correction; i = i + 1u) {
        let hitRecord: HitRecord = hitScene(currentRay);

        if !hitRecord.hit {
            color = color * getBackgroundColor(currentRay);
            break;
        }

        var bounceDir: vec3<f32>;
        let dir = normalize(currentRay.direction);
        switch (u32(hitRecord.material)) {
            // Lambertian
            case 0u: {
                bounceDir = scatter(dir, hitRecord.normal, randomSeed);
                if dot(bounceDir, hitRecord.normal) <= 0.0 {
                    return color * hitRecord.attenuation * getBackgroundColor(currentRay);
                }

                color = color * hitRecord.attenuation;
                break;
            }
            // Metal
            case 1u: {
                bounceDir = reflect(dir, hitRecord.normal);
                if dot(bounceDir, hitRecord.normal) <= 0.0 {
                    return color * hitRecord.attenuation * getBackgroundColor(currentRay);
                }

                color = color * hitRecord.attenuation;
                break;
            }
            // Dielectric
            case 2u: {
                let refractionIndex: f32 = select(1.5, 1.0 / 1.5, hitRecord.frontFace);

                let cosTheta: f32 = min(dot(-dir, hitRecord.normal), 1.0);
                let sinTheta: f32 = sqrt(1.0 - cosTheta * cosTheta);

                let cannotRefract: bool = refractionIndex * sinTheta > 1.0;

                if cannotRefract || reflectance(cosTheta, refractionIndex) > rand(hitRecord.p.xy) {
                    bounceDir = reflect(dir, hitRecord.normal);
                } else {
                    bounceDir = refract(dir, hitRecord.normal, refractionIndex);
                }

                color = color * hitRecord.attenuation;
                break;
            }
            // Gizmo
            case 3u: {
                let dot = dot(initialRay.direction, hitRecord.normal);
                if i == 0u && dot <= 0.2 && dot >= -0.2 {
                    return hitRecord.attenuation;
                }
                bounceDir = dir;
                correction = correction + 1u;
                break;
            }
            default: {
                bounceDir = scatter(dir, hitRecord.normal, randomSeed);
                color = color * hitRecord.attenuation;
                break;
            }
        }

        currentRay = Ray(hitRecord.p, bounceDir);
    }

    return color;
}

fn getBackgroundColor(ray: Ray) -> vec3<f32> {
    let bgColor: vec4<f32> = textureSampleLevel(skyTexture, skyTextureSampler, ray.direction, 0.0);
    return bgColor.rgb;
}

fn hitScene(ray: Ray) -> HitRecord {
    var hitRecord: HitRecord = HitRecord(
        false,
        0.0,
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0),
        false,
        vec3<f32>(0.0, 0.0, 0.0),
        0.0
    );

    for (var i = 0u; i < sphereData.sphereCount; i = i + 1u) {
        let sphere = sphereData.spheres[i];
        let objectHitRecord = hitSphere(ray, sphere);

        if !objectHitRecord.hit {
            continue;
        }

        if !hitRecord.hit || objectHitRecord.t < hitRecord.t {
            hitRecord = objectHitRecord;
        }
    }

    return hitRecord;
}

fn hitSphere(ray: Ray, sphere: Sphere) -> HitRecord {
    let centerToRayOrigin: vec3<f32> = ray.origin - sphere.center;
    let a: f32 = dot(ray.direction, ray.direction);
    let b: f32 = 2.0 * dot(ray.direction, centerToRayOrigin);
    let c: f32 = dot(centerToRayOrigin, centerToRayOrigin) - sphere.radius * sphere.radius;
    let discriminant: f32 = b * b - 4.0 * a * c;

    var hitRecord: HitRecord = HitRecord(
        false,
        0.0,
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0),
        false,
        sphere.albedo,
        sphere.material
    );

    if discriminant < 0.0 {
        return hitRecord;
    }

    var root = (-b - sqrt(discriminant)) / (2.0 * a);
    if root <= settings.tMin || root >= settings.tMax {
        root = (-b + sqrt(discriminant)) / (2.0 * a);
    }

    if root <= settings.tMin || root >= settings.tMax {
        return hitRecord;
    }

    hitRecord.hit = true;
    hitRecord.t = root;
    hitRecord.p = ray.origin + root * ray.direction;

    let outwardNormal: vec3<f32> = (hitRecord.p - sphere.center) / sphere.radius;
    hitRecord.frontFace = dot(ray.direction, outwardNormal) < 0.0;
    hitRecord.normal = select(-outwardNormal, outwardNormal, hitRecord.frontFace);

    if u32(sphere.material) == 3u {
        let dot = dot(ray.direction, hitRecord.normal);
        if !(dot <= 0.2 && dot >= -0.2) {
            hitRecord.hit = false;
        }
    }

    return hitRecord;
}

fn scatter(dir: vec3<f32 >, normal: vec3<f32>, seed: f32) -> vec3<f32> {
    var scatterDirection: vec3<f32> = normal + randomUnit(dir.xy * seed);

    if abs(scatterDirection.x) < 1e-8 && abs(scatterDirection.y) < 1e-8 && abs(scatterDirection.z) < 1e-8 {
        scatterDirection = normal;
    }

    return scatterDirection;
}

fn reflect(dir: vec3<f32 >, normal: vec3<f32>) -> vec3<f32> {
    let reflected: vec3<f32> = normalize(dir - 2.0 * dot(dir, normal) * normal);
    let fuzz: f32 = 0.0;

    return reflected + fuzz * randomUnit(dir.xy) * dot(reflected, normal);
}

fn reflectance(cosine: f32, refIdx: f32) -> f32 {
    let r0: f32 = (1.0 - refIdx) / (1.0 + refIdx);
    let r0Squared: f32 = r0 * r0;

    return r0Squared + (1.0 - r0Squared) * pow(1.0 - cosine, 5.0);
}

fn refract(dir: vec3<f32 >, normal: vec3<f32>, etaiOverEtat: f32) -> vec3<f32> {
    let cosTheta: f32 = min(dot(-dir, normal), 1.0);

    let rOutPerp: vec3<f32> = etaiOverEtat * (dir + cosTheta * normal);
    let rOutParallel: vec3<f32> = -sqrt(abs(1.0 - dot(rOutPerp, rOutPerp))) * normal;

    return rOutParallel + rOutPerp;
}
