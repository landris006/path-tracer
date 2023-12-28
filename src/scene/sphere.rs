use std::{cmp, usize};

use crate::MAX_NUMBER_OF_SPHERES;
use bytemuck::Zeroable;
use cgmath::{InnerSpace, Vector3};
use uuid::Uuid;

use super::{Material, Ray};

pub struct SphereDescriptor {
    pub center: Vector3<f32>,
    pub radius: f32,
    pub albedo: Vector3<f32>,
    pub material: Material,
}

#[derive(Debug)]
pub struct Sphere {
    pub uuid: uuid::Uuid,
    pub label: Option<String>,
    pub center: Vector3<f32>,
    pub radius: f32,
    pub albedo: Vector3<f32>,
    pub material: Material,
}

impl Sphere {
    pub fn new(sphere_descriptor: SphereDescriptor) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            label: None,
            center: sphere_descriptor.center,
            radius: sphere_descriptor.radius,
            albedo: sphere_descriptor.albedo,
            material: sphere_descriptor.material,
        }
    }

    pub fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let oc = ray.origin - self.center;
        let a = ray.direction.magnitude2();
        let half_b = oc.dot(ray.direction);
        let c = oc.magnitude2() - self.radius * self.radius;
        let discriminant = half_b * half_b - a * c;

        if discriminant > 0.0 {
            let root = discriminant.sqrt();

            let mut t = (-half_b - root) / a;
            if t < t_max && t > t_min {
                let point = ray.at(t);
                return Some(HitRecord {
                    point,
                    t,
                    sphere: self,
                });
            }

            t = (-half_b + root) / a;
            if t < t_max && t > t_min {
                let point = ray.at(t);
                return Some(HitRecord {
                    point,
                    t,
                    sphere: self,
                });
            }
        }

        None
    }
}

#[derive(Debug)]
pub struct HitRecord<'a> {
    pub point: Vector3<f32>,
    pub t: f32,
    pub sphere: &'a Sphere,
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
                Material::Gizmo => 3.0,
            },
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereDataBuffer {
    sphere_count: u32,
    _padding: [u32; 3],
    spheres: [SphereBuffer; MAX_NUMBER_OF_SPHERES as _],
}

impl From<&Vec<Sphere>> for SphereDataBuffer {
    fn from(spheres: &Vec<Sphere>) -> Self {
        let mut sphere_buffer = [SphereBuffer::zeroed(); MAX_NUMBER_OF_SPHERES as _];
        for (i, sphere) in spheres
            .iter()
            .take(MAX_NUMBER_OF_SPHERES as usize)
            .enumerate()
        {
            sphere_buffer[i] = SphereBuffer::from(sphere);
        }

        Self {
            sphere_count: cmp::min(spheres.len(), MAX_NUMBER_OF_SPHERES as usize) as u32,
            _padding: [0; 3],
            spheres: sphere_buffer,
        }
    }
}

