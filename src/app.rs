use std::time::Instant;

use cgmath::Vector3;

use wgpu::util::DeviceExt;
use winit::{event::WindowEvent, window::Window};

use crate::{
    camera::{Camera, CameraBuffer, CameraController},
    renderer::{Renderer, RendererDescriptor},
    scene::{Material, Scene, Sphere},
};

const NUMBER_OF_SAMPLES: usize = 16;

pub struct App {
    pub renderer: Renderer,

    start_time: Instant,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window_size: winit::dpi::PhysicalSize<u32>,

    scene: Scene,
    camera_controller: CameraController,

    last_frame_time: std::time::Instant,

    window: Window,
}

impl App {
    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn window_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.window_size
    }

    pub async fn new(window: Window) -> Self {
        let window_size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::TEXTURE_BINDING_ARRAY | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web, we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits {
                            max_texture_dimension_2d: 16384,
                            ..Default::default()
                        }
                    },
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(wgpu::TextureFormat::Rgba8Unorm);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // asd

        let renderer = Renderer::new(RendererDescriptor {
            window: &window,
            device: &device,
            queue: &queue,
            surface_config: &config,
            surface_format: &surface_format,
        });

        let camera = Camera {
            origin: Vector3::new(0.0, 0.0, 0.0),
            forward: Vector3::new(0.0, 0.0, -1.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            right: Vector3::new(1.0, 0.0, 0.0),
            focal_length: 1.0,
            vfov: 45.0,
        };

        let spheres = vec![
            Sphere {
                center: Vector3::new(0.0, 0.0, -1.0),
                radius: 0.5,
                albedo: Vector3::new(0.8, 0.3, 0.3),
                material: Material::Diffuse,
            },
            Sphere {
                center: Vector3::new(1.0, 0.0, -1.0),
                radius: 0.5,
                albedo: Vector3::new(1.0, 1.0, 1.0),
                material: Material::Dielectric,
            },
            Sphere {
                center: Vector3::new(-1.0, 0.0, -1.0),
                radius: 0.5,
                albedo: Vector3::new(1.0, 1.0, 1.0),
                material: Material::Dielectric,
            },
            Sphere {
                center: Vector3::new(0.0, 1.0, -1.0),
                radius: 0.5,
                albedo: Vector3::new(0.8, 0.3, 0.3),
                material: Material::Diffuse,
            },
            Sphere {
                center: Vector3::new(0.0, 2.0, -1.0),
                radius: 0.5,
                albedo: Vector3::new(0.8, 0.3, 0.3),
                material: Material::Metal,
            },
            Sphere {
                center: Vector3::new(0.0, -100.5, -1.0),
                radius: 100.0,
                albedo: Vector3::new(0.8, 0.8, 0.0),
                material: Material::Diffuse,
            },
        ];

        let scene = Scene { camera, spheres };

        Self {
            surface,
            device,
            queue,
            config,
            window_size,

            start_time: Instant::now(),
            scene,
            camera_controller: CameraController::new(),
            last_frame_time: Instant::now(),
            renderer,
            window,
        }
    }

    pub fn update(&mut self) {
        let now = std::time::Instant::now();
        let delta = now - self.last_frame_time;
        self.last_frame_time = now;

        self.camera_controller
            .update_camera(&mut self.scene.camera, delta.as_secs_f32());
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let mut output = self.surface.get_current_texture()?;

        self.renderer.render(
            &mut output,
            &self.scene,
            &self.device,
            &self.queue,
            &self.window,
        )?;

        output.present();

        Ok(())
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.window_size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) {
        self.camera_controller.input(event, &mut self.window);
    }
}
