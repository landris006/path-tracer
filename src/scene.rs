use cgmath::Vector3;

#[derive(Debug)]
pub struct Sphere {
    pub center: Vector3<f32>,
    pub radius: f32,
    pub albedo: Vector3<f32>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereBuffer {
    center: [f32; 3],
    radius: f32,
    albedo: [f32; 3],
    _padding: u32,
}
impl From<&Sphere> for SphereBuffer {
    fn from(sphere: &Sphere) -> Self {
        Self {
            center: sphere.center.into(),
            radius: sphere.radius,
            albedo: sphere.albedo.into(),
            _padding: 0,
        }
    }
}
