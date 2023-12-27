use std::{cmp, usize};

use crate::{camera::Ray, MAX_NUMBER_OF_SPHERES};
use bytemuck::Zeroable;
use cgmath::{InnerSpace, Vector3};
use uuid::Uuid;

use crate::camera::Camera;

pub struct Scene {
    pub camera: Camera,
    pub spheres: Vec<Sphere>,
    pub selected_sphere: Option<Uuid>,
}

impl Scene {
    pub fn new(spheres: Vec<Sphere>, camera: Camera) -> Self {
        Self {
            camera,
            spheres,
            selected_sphere: None,
        }
    }

    pub fn render_ui(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        ui.collapsing("Scene", |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Add Sphere")
                    .on_hover_text("Add a sphere to the scene")
                    .clicked()
                {
                    self.spheres.push(Sphere::new(SphereDescriptor {
                        center: Vector3::new(0.0, 0.0, 0.0),
                        radius: 1.0,
                        albedo: Vector3::new(0.5, 0.5, 0.5),
                        material: Material::Diffuse,
                    }));
                }

                if ui
                    .button("Remove Sphere")
                    .on_hover_text("Remove the last sphere from the scene")
                    .clicked()
                {
                    self.spheres.pop();
                }
            });
            ui.separator();

            for (i, sphere) in self.spheres.iter_mut().enumerate() {
                ui.collapsing(format!("Sphere {}", i), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Center");
                        ui.add(egui::DragValue::new(&mut sphere.center.x).speed(0.1));
                        ui.add(egui::DragValue::new(&mut sphere.center.y).speed(0.1));
                        ui.add(egui::DragValue::new(&mut sphere.center.z).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Radius");
                        ui.add(egui::DragValue::new(&mut sphere.radius).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Albedo");
                        ui.add(egui::DragValue::new(&mut sphere.albedo.x));
                        ui.add(egui::DragValue::new(&mut sphere.albedo.y));
                        ui.add(egui::DragValue::new(&mut sphere.albedo.z));

                        let mut color: [f32; 3] = sphere.albedo.into();
                        ui.color_edit_button_rgb(&mut color);
                        sphere.albedo = color.into();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Material");
                        ui.radio_value(&mut sphere.material, Material::Diffuse, "Diffuse");
                        ui.radio_value(&mut sphere.material, Material::Metal, "Metal");
                        ui.radio_value(&mut sphere.material, Material::Dielectric, "Dielectric");
                    });
                });
            }
        });

        if let Some(selected_sphere) = self.selected_sphere {
            if let Some(sphere) = self.spheres.iter_mut().find(|s| s.uuid == selected_sphere) {
                egui::Window::new("Selected Sphere")
                    .default_pos(egui::Pos2::new(400.0, 400.0))
                    .resizable(true)
                    .show(context, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Center");
                            ui.add(egui::DragValue::new(&mut sphere.center.x).speed(0.1));
                            ui.add(egui::DragValue::new(&mut sphere.center.y).speed(0.1));
                            ui.add(egui::DragValue::new(&mut sphere.center.z).speed(0.1));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Radius");
                            ui.add(egui::DragValue::new(&mut sphere.radius).speed(0.1));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Albedo");
                            ui.add(egui::DragValue::new(&mut sphere.albedo.x));
                            ui.add(egui::DragValue::new(&mut sphere.albedo.y));
                            ui.add(egui::DragValue::new(&mut sphere.albedo.z));

                            let mut color: [f32; 3] = sphere.albedo.into();
                            ui.color_edit_button_rgb(&mut color);
                            sphere.albedo = color.into();
                        });
                        ui.horizontal(|ui| {
                            ui.label("Material");
                            ui.radio_value(&mut sphere.material, Material::Diffuse, "Diffuse");
                            ui.radio_value(&mut sphere.material, Material::Metal, "Metal");
                            ui.radio_value(
                                &mut sphere.material,
                                Material::Dielectric,
                                "Dielectric",
                            );
                        });
                    });
            }
        }
    }

    pub fn hit_closest_sphere(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let mut closest_so_far = t_max;
        let mut closest_hit: Option<HitRecord> = None;

        for sphere in self.spheres.iter() {
            if let Some(hit) = sphere.hit(ray, t_min, closest_so_far) {
                closest_so_far = hit.t;
                closest_hit = Some(hit);
            }
        }

        closest_hit
    }

    pub fn update(&mut self) -> Option<()> {
        let selected_sphere = self.selected_sphere?;
        let mut spheres_iter = self.spheres.iter_mut();
        let sphere = spheres_iter.find(|s| s.uuid == selected_sphere)?;
        let gizmo = spheres_iter.find(|s| s.label == Some("selected_sphere_gizmo".to_string()))?;

        gizmo.center = sphere.center;
        gizmo.radius = sphere.radius + 0.01;

        Some(())
    }
}

#[derive(Debug, PartialEq)]
pub enum Material {
    Diffuse,
    Metal,
    Dielectric,
    Gizmo,
}

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
