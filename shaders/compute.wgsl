struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
};

struct Sphere {
  center: vec3<f32>,
  radius: f32,
}

struct HitRecord {
    hit: bool,
    t: f32,
    p: vec3<f32>,
    color: vec3<f32>,
    normal: vec3<f32>,
    frontFace: bool,
}

@group(0) @binding(1) var outputTex: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) globalIndex: vec3<u32>) {
    let screen_size: vec2<f32> = vec2<f32>(textureDimensions(outputTex));

    let aspectRatio: f32 = screen_size.x / screen_size.y;
    let viewPortHeight: f32 = 2.0;
    let viewPortWidth: f32 = aspectRatio * viewPortHeight;

    let viewPortU: vec3<f32> = vec3<f32>(viewPortWidth, 0.0, 0.0);
    let viewPortV: vec3<f32> = vec3<f32>(0.0, -viewPortHeight, 0.0);

    let focalLength: f32 = 1.0;
    let eye = vec3<f32>(0.0, 0.0, 0.0);
    let forwards: vec3<f32> = vec3<f32>(0.0, 0.0, -1.0);
    let right: vec3<f32> = vec3<f32>(1.0, 0.0, 0.0);
    let up: vec3<f32> = vec3<f32>(0.0, 1.0, 0.0);

    let pixelDeltaU = viewPortU / screen_size.x;
    let pixelDeltaV = viewPortV / screen_size.y;

    let upper_left: vec3<f32> = eye + focalLength * forwards - 0.5 * (viewPortU + viewPortV);
    let pixel00Location: vec3<f32> = upper_left + 0.5 * (pixelDeltaU + pixelDeltaV);

    let pixelLocation: vec3<f32> = pixel00Location + f32(globalIndex.x) * pixelDeltaU + f32(globalIndex.y) * pixelDeltaV;

    let ray: Ray = Ray(eye, pixelLocation - eye);

    let fragColor: vec4<f32> = vec4<f32>(rayColor(ray), 1.0);

    textureStore(outputTex, vec2<i32>(globalIndex.xy), fragColor);
}

fn rayColor(ray: Ray) -> vec3<f32> {
    let sphere = Sphere(vec3<f32>(0.0, 0.0, -1.0), 0.5);
    let hitRecord = hitSphere(ray, sphere);

    if hitRecord.hit {
        return 0.5 * (hitRecord.normal + vec3<f32>(1.0, 1.0, 1.0));
    }

    let unitDirection: vec3<f32> = normalize(ray.direction);
    let a: f32 = 0.5 * (ray.direction.y + 1.0);
    return (1.0 - a) * vec3<f32>(1.0, 1.0, 1.0) + a * vec3<f32>(0.5, 0.7, 1.0);
}

fn hitSphere(ray: Ray, sphere: Sphere) -> HitRecord {
    let centerToRayOrigin: vec3<f32> = ray.origin - sphere.center;
    let a: f32 = dot(ray.direction, ray.direction);
    let b: f32 = 2.0 * dot(ray.direction, centerToRayOrigin);
    let c: f32 = dot(centerToRayOrigin, centerToRayOrigin) - sphere.radius * sphere.radius;
    let discriminant: f32 = b * b - 4.0 * a * c;

    var hitRecord: HitRecord = HitRecord(false, 0.0, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 0.0), false);

    if discriminant < 0.0 {
        return hitRecord;
    }

    let tMin = 0.001;
    let tMax = 1000.0;

    var root = (-b - sqrt(discriminant)) / (2.0 * a);
    if root <= tMin || root >= tMax {
        root = (-b + sqrt(discriminant)) / (2.0 * a);
    }

    if root <= tMin || root >= tMax {
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
