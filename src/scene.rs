use cgmath::Vector3;
use egui::Color32;
use egui_winit_platform::Platform;

use crate::camera::Camera;

pub struct Scene {
    pub camera: Camera,
    pub spheres: Vec<Sphere>,
}

impl Scene {
    pub fn render_ui(&mut self, platform: &mut Platform) {
        egui::Window::new("Scene")
            .resizable(true)
            .show(&platform.context(), |ui| {
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
                            ui.radio_value(
                                &mut sphere.material,
                                Material::Dielectric,
                                "Dielectric",
                            );
                        });
                    });
                }
            });
    }
}

#[derive(Debug, PartialEq)]
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
