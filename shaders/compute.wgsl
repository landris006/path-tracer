//!include "utils.wgsl"

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
};

struct Sphere {
  center: vec3<f32>,
  radius: f32,
  albedo: vec3<f32>,
  material: u32,
}

struct Camera {
    origin: vec3<f32>,
    focalLength: f32,
    forward: vec3<f32>,
    _padding: u32,
    right: vec3<f32>,
    _padding2: u32,
    up: vec3<f32>,
    _padding3: u32,
}

struct HitRecord {
    hit: bool,
    t: f32,
    p: vec3<f32>,
    normal: vec3<f32>,
    frontFace: bool,
    attenuation: vec3<f32>,
    material: u32,
}

@group(0) @binding(0) var outputTex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> spheres: array<Sphere>;
@group(0) @binding(3) var<uniform> time: f32;

const T_MIN: f32 = 0.001;
const T_MAX: f32 = 1000.0;
const MAX_DEPTH: u32 = 30u;
const SAMPLE_SIZE: u32 = 30u;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) threadId: vec3<u32>) {
    let screen_size: vec2<u32> = vec2<u32>(textureDimensions(outputTex));

    if threadId.x >= screen_size.x || threadId.y >= screen_size.y {
        return;
    }

    let aspectRatio: f32 = f32(screen_size.x) / f32(screen_size.y);
    let viewPortHeight: f32 = 2.0;
    let viewPortWidth: f32 = aspectRatio * viewPortHeight;

    let viewPortU: vec3<f32> = vec3<f32>(viewPortWidth, 0.0, 0.0);
    let viewPortV: vec3<f32> = vec3<f32>(0.0, -viewPortHeight, 0.0);

    let pixelDeltaU = viewPortU / f32(screen_size.x);
    let pixelDeltaV = viewPortV / f32(screen_size.y);

    let upper_left: vec3<f32> = camera.origin + camera.focalLength * camera.forward - 0.5 * (viewPortU + viewPortV);
    let pixel00Location: vec3<f32> = upper_left + 0.5 * (pixelDeltaU + pixelDeltaV);

    let pixelLocation: vec3<f32> = pixel00Location + f32(threadId.x) * pixelDeltaU + f32(threadId.y) * pixelDeltaV;

    var color: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0u; i < SAMPLE_SIZE; i = i + 1u) {
        let px = -0.5 + rand(pixelLocation.xy * f32(i));
        let py = -0.5 + rand(pixelLocation.yx * f32(i) * px);

        let sample: vec3<f32> = pixelDeltaU * px + pixelDeltaV * py;
        let sampleLocation: vec3<f32> = pixelLocation + sample;

        let ray: Ray = Ray(camera.origin, sampleLocation - camera.origin);

        color = color + rayColor(ray);
    }

    color = color / f32(SAMPLE_SIZE);

    let fragColor: vec4<f32> = vec4<f32>(color, 1.0);
    textureStore(outputTex, vec2<i32>(threadId.xy), fragColor);
}

fn rayColor(initialRay: Ray) -> vec3<f32> {
    var numberOfBounces: u32 = 0u;
    var color = vec3<f32>(1.0, 1.0, 1.0);

    var currentRay: Ray = initialRay;
    for (var i = 0u; i < MAX_DEPTH; i = i + 1u) {
        numberOfBounces = numberOfBounces + 1u;
        let hitRecord: HitRecord = hitScene(currentRay);

        if !hitRecord.hit {
            break;
        }

        var bounceDir: vec3<f32>;
        switch (hitRecord.material) {
            // Lambertian
            case 0u: {
                bounceDir = scatter(currentRay.direction, hitRecord.normal);
                if dot(bounceDir, hitRecord.normal) <= 0.0 {
                    return color * hitRecord.attenuation;
                }
                break;
            }
            // Metal
            case 1u: {
                bounceDir = reflect(currentRay.direction, hitRecord.normal);
                if dot(bounceDir, hitRecord.normal) <= 0.0 {
                    return color * hitRecord.attenuation;
                }
                break;
            }
            default: {
                bounceDir = scatter(currentRay.direction, hitRecord.normal);
                break;
            }
        }

        currentRay = Ray(hitRecord.p, bounceDir);
        color = color * hitRecord.attenuation;
    }

    return color * getBackgroundColor(currentRay);
}

fn getBackgroundColor(ray: Ray) -> vec3<f32> {
    let unitDirection: vec3<f32> = normalize(ray.direction);
    let a: f32 = 0.5 * (unitDirection.y + 1.0);
    return (1.0 - a) * vec3<f32>(1.0, 1.0, 1.0) + a * vec3<f32>(0.5, 0.7, 1.0);
}

fn hitScene(ray: Ray) -> HitRecord {
    var hitRecord: HitRecord = HitRecord(
        false,
        0.0,
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0),
        false,
        vec3<f32>(0.0, 0.0, 0.0),
        0u
    );

    for (var i = 0u; i < arrayLength(&spheres); i = i + 1u) {
        let sphere = spheres[i];
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
    if root <= T_MIN || root >= T_MAX {
        root = (-b + sqrt(discriminant)) / (2.0 * a);
    }

    if root <= T_MIN || root >= T_MAX {
        return hitRecord;
    }

    hitRecord.hit = true;
    hitRecord.t = root;
    hitRecord.p = ray.origin + root * ray.direction;

    let outwardNormal: vec3<f32> = (hitRecord.p - sphere.center) / sphere.radius;
    hitRecord.frontFace = dot(ray.direction, outwardNormal) < 0.0;
    hitRecord.normal = select(-outwardNormal, outwardNormal, hitRecord.frontFace);

    return hitRecord;
}

fn scatter(dir: vec3<f32 >, normal: vec3<f32>) -> vec3<f32> {
    var scatterDirection: vec3<f32> = normal + randomUnit(dir.xy);

    if abs(scatterDirection.x) < 1e-8 && abs(scatterDirection.y) < 1e-8 && abs(scatterDirection.z) < 1e-8 {
        scatterDirection = normal;
    }

    return scatterDirection;
}

fn reflect(dir: vec3<f32 >, normal: vec3<f32>) -> vec3<f32> {
    let reflected: vec3<f32> = normalize(dir - 2.0 * dot(dir, normal) * normal);
    let fuzz: f32 = 0.2;

    return reflected + fuzz * randomUnit(dir.xy) * dot(reflected, normal);
}
