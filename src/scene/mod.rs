use cgmath::Vector3;
use egui::Response;
use uuid::Uuid;

mod bvh;
mod camera;
mod sphere;

pub use camera::*;
pub use sphere::*;

use crate::{model::Triangle, renderer::Renderer};

use self::bvh::Bvh;

#[derive(Debug, PartialEq)]
pub enum Material {
    Diffuse,
    Metal,
    Dielectric,
    Gizmo,
}

pub struct Scene {
    pub camera: Camera,
    pub spheres: Vec<Sphere>,
    pub selected_sphere: Option<Uuid>,
    pub triangles: Vec<Triangle>,
    pub bvh: Bvh,
}

impl Scene {
    pub fn new(spheres: Vec<Sphere>, triangles: Vec<Triangle>, camera: Camera) -> Self {
        Self {
            camera,
            spheres,
            selected_sphere: None,
            bvh: Bvh::from_triangles(&triangles),
            triangles,
        }
    }

    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        context: &egui::Context,
        renderer: &mut Renderer,
    ) {
        let mut responses: Vec<Response> = Vec::new();

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
                    renderer.progressive_rendering.reset_ready_samples();
                }

                if ui
                    .button("Remove Sphere")
                    .on_hover_text("Remove the last sphere from the scene")
                    .clicked()
                {
                    self.spheres.pop();
                    renderer.progressive_rendering.reset_ready_samples();
                }
            });
            ui.separator();

            for (i, sphere) in self.spheres.iter_mut().enumerate() {
                ui.collapsing(format!("Sphere {}", i), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Center");
                        responses.extend([
                            ui.add(egui::DragValue::new(&mut sphere.center.x).speed(0.1)),
                            ui.add(egui::DragValue::new(&mut sphere.center.y).speed(0.1)),
                            ui.add(egui::DragValue::new(&mut sphere.center.z).speed(0.1)),
                        ]);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Radius");
                        responses.push(ui.add(egui::DragValue::new(&mut sphere.radius).speed(0.1)));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Albedo");
                        responses.extend([
                            ui.add(egui::DragValue::new(&mut sphere.albedo.x)),
                            ui.add(egui::DragValue::new(&mut sphere.albedo.y)),
                            ui.add(egui::DragValue::new(&mut sphere.albedo.z)),
                        ]);

                        let mut color: [f32; 3] = sphere.albedo.into();
                        responses.push(ui.color_edit_button_rgb(&mut color));
                        sphere.albedo = color.into();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Material");
                        responses.extend([
                            ui.radio_value(&mut sphere.material, Material::Diffuse, "Diffuse"),
                            ui.radio_value(&mut sphere.material, Material::Metal, "Metal"),
                            ui.radio_value(
                                &mut sphere.material,
                                Material::Dielectric,
                                "Dielectric",
                            ),
                        ]);
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
                            responses.extend([
                                ui.add(egui::DragValue::new(&mut sphere.center.x).speed(0.1)),
                                ui.add(egui::DragValue::new(&mut sphere.center.y).speed(0.1)),
                                ui.add(egui::DragValue::new(&mut sphere.center.z).speed(0.1)),
                            ]);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Radius");
                            responses
                                .push(ui.add(egui::DragValue::new(&mut sphere.radius).speed(0.1)));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Albedo");
                            responses.extend([
                                ui.add(egui::DragValue::new(&mut sphere.albedo.x)),
                                ui.add(egui::DragValue::new(&mut sphere.albedo.y)),
                                ui.add(egui::DragValue::new(&mut sphere.albedo.z)),
                            ]);

                            let mut color: [f32; 3] = sphere.albedo.into();
                            responses.push(ui.color_edit_button_rgb(&mut color));
                            sphere.albedo = color.into();
                        });
                        ui.horizontal(|ui| {
                            ui.label("Material");
                            responses.extend([
                                ui.radio_value(&mut sphere.material, Material::Diffuse, "Diffuse"),
                                ui.radio_value(&mut sphere.material, Material::Metal, "Metal"),
                                ui.radio_value(
                                    &mut sphere.material,
                                    Material::Dielectric,
                                    "Dielectric",
                                ),
                            ]);
                        });
                    });
            }
        }

        if responses.iter().any(|r| r.changed()) {
            renderer.progressive_rendering.reset_ready_samples();
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

