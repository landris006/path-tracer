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
  a: vec3<f32>,
  _pad0: f32,
  b: vec3<f32>,
  _pad1: f32,
  c: vec3<f32>,
  _pad2: f32,
  an: vec3<f32>,
  _pad3: f32,
  bn: vec3<f32>,
  _pad4: f32,
  cn: vec3<f32>,
  _pad5: f32,
}

struct Node {
  minCorner: vec3<f32>,
  leftChildIndex: u32,
  maxCorner: vec3<f32>,
  triangleCount: u32,
}

struct SphereData {
  sphereCount: u32,
  spheres: array<Sphere>,
}

@group(0) @binding(0) var outputTex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> sphereData: SphereData;
@group(0) @binding(3) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(4) var<storage, read> triangleIndices: array<u32>;
@group(0) @binding(5) var<storage, read> bvhNodes: array<Node>;
@group(0) @binding(6) var<uniform> time: u32;
@group(0) @binding(7) var skyTexture: texture_cube<f32>;
@group(0) @binding(8) var skyTextureSampler: sampler;
@group(0) @binding(9) var<uniform> settings: Settings;

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


    // for (var i = 0u; i < 1000u; i++) {
    //     let triangle = triangles[(triangleIndices[i])];
    //     let objectHitRecord = hitTriangle(ray, triangle);

    //     if !objectHitRecord.hit {
    //                 continue;
    //     }

    //     if !hitRecord.hit || objectHitRecord.t < hitRecord.t {
    //         hitRecord = objectHitRecord;
    //     }
    // }

    var node: Node = bvhNodes[0u];
    var stack: array<Node, 15>;
    var stackLocation: u32 = 0u;
    var nearestHit: f32 = 9999.0;

    while true {
        var contents: u32 = u32(node.leftChildIndex);

        if node.triangleCount == 0u {
            var child1: Node = bvhNodes[contents];
            var child2: Node = bvhNodes[contents + 1u];

            var distance1: f32 = hitAabb(ray, child1);
            var distance2: f32 = hitAabb(ray, child2);
            if distance1 > distance2 {
                var tempDist: f32 = distance1;
                distance1 = distance2;
                distance2 = tempDist;

                var tempChild: Node = child1;
                child1 = child2;
                child2 = tempChild;
            }

            if distance1 > nearestHit {
                if stackLocation == 0u {
                    break;
                } else {
                    stackLocation -= 1u;
                    node = stack[stackLocation];
                }
            } else {
                node = child1;
                if distance2 < nearestHit {
                    stack[stackLocation] = child2;
                    stackLocation += 1u;
                }
            }
        } else {
            for (var i = 0u; i < node.triangleCount; i++) {
                let triangle = triangles[(triangleIndices[i + contents])];
                let objectHitRecord = hitTriangle(ray, triangle);

                if !objectHitRecord.hit {
                    continue;
                }

                if !hitRecord.hit || objectHitRecord.t < hitRecord.t {
                    hitRecord = objectHitRecord;
                    nearestHit = hitRecord.t;
                }
            }

            if stackLocation == 0u {
                break;
            } else {
                stackLocation -= 1u;
                node = stack[stackLocation];
            }
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

fn hitTriangle(ray: Ray, triangle: Triangle) -> HitRecord {
    let edge1: vec3<f32> = triangle.b - triangle.a;
    let edge2: vec3<f32> = triangle.c - triangle.a;

    let h: vec3<f32> = cross(ray.direction, edge2);
    let a: f32 = dot(edge1, h);

    var hitRecord: HitRecord = HitRecord(
        false,
        0.0,
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0),
        false,
        vec3<f32>(1.0, 1.0, 1.0),
        2.0,
    );

    if a > -0.00001 && a < 0.00001 {
        return hitRecord; // This ray is parallel to this triangle.
    }

    let f: f32 = 1.0 / a;
    let s: vec3<f32> = ray.origin - triangle.a;
    let u: f32 = f * dot(s, h);

    if u < 0.0 || u > 1.0 {
        return hitRecord;
    }

    let q: vec3<f32> = cross(s, edge1);
    let v: f32 = f * dot(ray.direction, q);

    if v < 0.0 || u + v > 1.0 {
        return hitRecord;
    }

    let t: f32 = f * dot(edge2, q);

    if t <= settings.tMin || t >= settings.tMax {
        return hitRecord;
    }

    hitRecord.hit = true;
    hitRecord.t = t;
    hitRecord.p = ray.origin + t * ray.direction;

    let barycentric: vec3<f32> = vec3<f32>(1.0 - u - v, u, v);
    let normal: vec3<f32> = normalize(triangle.an * barycentric.x + triangle.bn * barycentric.y + triangle.cn * barycentric.z);

    let outwardNormal: vec3<f32> = normalize(triangle.an * barycentric.x + triangle.bn * barycentric.y + triangle.cn * barycentric.z);
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

fn hitAabb(ray: Ray, node: Node) -> f32 {
    var inverseDir: vec3<f32> = vec3<f32>(1.0) / ray.direction;
    var t1: vec3<f32> = (node.minCorner - ray.origin) * inverseDir;
    var t2: vec3<f32> = (node.maxCorner - ray.origin) * inverseDir;
    var tMin: vec3<f32> = min(t1, t2);
    var tMax: vec3<f32> = max(t1, t2);

    var t_min: f32 = max(max(tMin.x, tMin.y), tMin.z);
    var t_max: f32 = min(min(tMax.x, tMax.y), tMax.z);

    if t_min > t_max || t_max < 0.0 {
        return 99999.0;
    } else {
        return t_min;
    }
}
