use cgmath::{InnerSpace, Vector3};

use crate::model::Triangle;

use super::Material;

pub struct Plane {
    pub q: Vector3<f32>,
    pub u: Vector3<f32>,
    pub v: Vector3<f32>,
    pub albedo: Vector3<f32>,
    pub material: Material,
}

impl Plane {
    pub fn triangles(self) -> Vec<Triangle> {
        let normal = self.normal();
        let triangle1 = Triangle {
            a: self.q,
            b: (self.q + self.u),
            c: (self.q + self.v),
            na: normal,
            nb: normal,
            nc: normal,
            albedo: self.albedo,
            material: self.material,
        };

        let triangle2 = Triangle {
            a: (self.q + self.u + self.v),
            b: (self.q + self.u),
            c: (self.q + self.v),
            na: normal,
            nb: normal,
            nc: normal,
            albedo: self.albedo,
            material: self.material,
        };

        vec![triangle1, triangle2]
    }

    pub fn normal(&self) -> Vector3<f32> {
        self.u.cross(self.v).normalize()
    }
}

