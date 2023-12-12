use cgmath::Vector3;

#[derive(Debug)]
pub enum Material {
    Diffuse,
    Metal,
    Dielectric,
}

#[derive(Debug)]
pub struct Sphere {
    pub center: Vector3<f32>,
    pub radius: f32,
    pub albedo: Vector3<f32>,
    pub material: Material,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereBuffer {
    center: [f32; 3],
    radius: f32,
    albedo: [f32; 3],
    material: f32,
}
impl From<&Sphere> for SphereBuffer {
    fn from(sphere: &Sphere) -> Self {
        Self {
            center: sphere.center.into(),
            radius: sphere.radius,
            albedo: sphere.albedo.into(),
            material: match sphere.material {
                Material::Diffuse => 0.0,
                Material::Metal => 1.0,
                Material::Dielectric => 2.0,
            },
        }
    }
}
