//!include "utils.wgsl"

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
};

struct Sphere {
  center: vec3<f32>,
  radius: f32,
  albedo: vec3<f32>,
  material: f32,
}

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
  maxBounces: u32,
  tMin: f32,
  tMax: f32,
}

@group(0) @binding(0) var outputTex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> spheres: array<Sphere>;
@group(0) @binding(3) var<uniform> time: u32;
@group(0) @binding(4) var skyTexture: texture_2d<f32>;
@group(0) @binding(5) var<uniform> settings: Settings;

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
    var numberOfBounces: u32 = 0u;
    var color = vec3<f32>(1.0, 1.0, 1.0);
    let randomSeed = hybridTaus(randomState).value;

    var currentRay: Ray = initialRay;
    for (var i = 0u; i < settings.maxBounces; i = i + 1u) {
        numberOfBounces = numberOfBounces + 1u;
        let hitRecord: HitRecord = hitScene(currentRay);

        if !hitRecord.hit {
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
                break;
            }
            // Metal
            case 1u: {
                bounceDir = reflect(dir, hitRecord.normal);
                if dot(bounceDir, hitRecord.normal) <= 0.0 {
                    return color * hitRecord.attenuation * getBackgroundColor(currentRay);
                }
                break;
            }
            // Dielectric
            case 2u: {
                let refractionRatio: f32 = select(1.5, 1.0 / 1.5, hitRecord.frontFace);

                let cosTheta: f32 = min(dot(-dir, hitRecord.normal), 1.0);
                let sinTheta: f32 = sqrt(1.0 - cosTheta * cosTheta);

                let cannotRefract: bool = refractionRatio * sinTheta > 1.0;

                if cannotRefract || reflectance(cosTheta, refractionRatio) > rand(hitRecord.p.xy) {
                    bounceDir = reflect(dir, hitRecord.normal);
                } else {
                    bounceDir = refract(dir, hitRecord.normal, refractionRatio);
                }

                break;
            }
            default: {
                bounceDir = scatter(dir, hitRecord.normal, randomSeed);
                break;
            }
        }

        currentRay = Ray(hitRecord.p, bounceDir);
        color = color * hitRecord.attenuation;
    }

    return color * getBackgroundColor(currentRay);
}

fn getBackgroundColor(ray: Ray) -> vec3<f32> {
    var azimuth: f32 = atan2(ray.direction.z, ray.direction.x);
    var inclination: f32 = acos(ray.direction.y);

// Map azimuth and inclination to texture coordinates
    var textureCoords: vec2<f32> = vec2<f32>(
        0.5 + azimuth / (2.0 * 3.14159265359), // Map azimuth to the X-axis (range from 0 to 1)
        inclination / 3.14159265359 // Map inclination to the Y-axis (range from 0 to 1)
    );

    let textureDimension: vec2<u32> = textureDimensions(skyTexture);

    var textureCoordsU32: vec2<u32> = vec2<u32>(
        u32(textureCoords.x * f32(textureDimension.x - 1u)),
        u32(textureCoords.y * f32(textureDimension.y - 1u))
    );

    let bgColor: vec4<f32> = textureLoad(skyTexture, textureCoordsU32, i32(0));

    return bgColor.xyz;
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
