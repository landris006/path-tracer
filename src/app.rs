use std::time::Instant;

use cgmath::Vector3;

use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use crate::{
    camera::{Camera, CameraController, Ray},
    renderer::Renderer,
    scene::{HitRecord, Material, Scene, Sphere, SphereDescriptor},
    ui::Ui,
};

pub struct App {
    pub renderer: Renderer,
    ui: Ui,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window_size: winit::dpi::PhysicalSize<u32>,
    cursor_ray: Ray,

    scene: Scene,
    camera_controller: CameraController,

    start_time: Instant,
    last_frame_time: std::time::Instant,
    frame_times: Vec<u128>,

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
                            max_sampled_textures_per_shader_stage: 256,
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

        let renderer = Renderer::new(&device, &queue, &config);

        let camera = Camera::new();

        let spheres = vec![
            Sphere::new(SphereDescriptor {
                center: Vector3::new(0.0, 0.0, -1.0),
                radius: 0.5,
                albedo: Vector3::new(0.8, 0.3, 0.3),
                material: Material::Diffuse,
            }),
            Sphere::new(SphereDescriptor {
                center: Vector3::new(1.0, 0.0, -1.0),
                radius: 0.5,
                albedo: Vector3::new(1.0, 1.0, 1.0),
                material: Material::Dielectric,
            }),
            Sphere::new(SphereDescriptor {
                center: Vector3::new(-1.0, 0.0, -1.0),
                radius: 0.5,
                albedo: Vector3::new(1.0, 1.0, 1.0),
                material: Material::Dielectric,
            }),
            Sphere::new(SphereDescriptor {
                center: Vector3::new(0.0, 1.0, -1.0),
                radius: 0.5,
                albedo: Vector3::new(0.8, 0.3, 0.3),
                material: Material::Diffuse,
            }),
            Sphere::new(SphereDescriptor {
                center: Vector3::new(0.0, 2.0, -1.0),
                radius: 0.5,
                albedo: Vector3::new(0.8, 0.3, 0.3),
                material: Material::Metal,
            }),
            Sphere::new(SphereDescriptor {
                center: Vector3::new(0.0, -100.5, -1.0),
                radius: 100.0,
                albedo: Vector3::new(0.8, 0.8, 0.0),
                material: Material::Diffuse,
            }),
        ];

        let ui = Ui::new(&window, &device, surface_format);
        let scene = Scene::new(spheres, camera);

        Self {
            surface,
            device,
            queue,
            config,
            window_size,
            ui,
            scene,
            camera_controller: CameraController::new(),
            start_time: Instant::now(),
            last_frame_time: Instant::now(),
            frame_times: Vec::new(),
            cursor_ray: Ray {
                origin: Vector3::new(0.0, 0.0, 0.0),
                direction: Vector3::new(0.0, 0.0, -1.0),
            },
            renderer,
            window,
        }
    }

    fn setup_ui(&mut self) {
        self.ui
            .begin_new_frame(self.start_time.elapsed().as_secs_f64());

        let avg_frame_time = self.frame_times.iter().sum::<u128>() / self.frame_times.len() as u128;

        let platform = self.ui.platform_mut();

        egui::Window::new("Info")
            .resizable(true)
            .show(&platform.context(), |ui| {
                ui.add(egui::Label::new(format!(
                    "Frame time: {}ms ({} FPS)",
                    avg_frame_time,
                    1000 / avg_frame_time
                )));
            });

        egui::Window::new("Camera settings")
            .default_open(false)
            .resizable(true)
            .show(&platform.context(), |ui| {
                ui.label("Origin");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.scene.camera.origin.x).speed(0.1));
                    ui.add(egui::DragValue::new(&mut self.scene.camera.origin.y).speed(0.1));
                    ui.add(egui::DragValue::new(&mut self.scene.camera.origin.z).speed(0.1));
                });
                ui.label("Look at");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.scene.camera.forward.x).speed(0.1));
                    ui.add(egui::DragValue::new(&mut self.scene.camera.forward.y).speed(0.1));
                    ui.add(egui::DragValue::new(&mut self.scene.camera.forward.z).speed(0.1));
                });
                ui.label("Vertical FOV");
                ui.add(egui::Slider::new(&mut self.scene.camera.vfov, 0.0..=180.0));
                ui.label("Speed");
                ui.add(egui::Slider::new(
                    &mut self.camera_controller.speed,
                    0.0..=10.0,
                ));
            });

        self.renderer
            .render_ui(platform, self.scene.camera.moved_recently());
        self.scene.render_ui(platform);
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let delta = now - self.last_frame_time;
        self.last_frame_time = now;

        self.frame_times.push(delta.as_millis());
        if self.frame_times.len() > 100 {
            self.frame_times.remove(0);
        }

        self.camera_controller
            .update_camera(&mut self.scene.camera, delta.as_secs_f32());
        self.scene.update();
    }

    pub fn ui_input(&mut self, event: &Event<()>) {
        self.ui.handle_event(event);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.setup_ui();

        let mut output = self.surface.get_current_texture()?;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.renderer
            .render(&mut output, &mut encoder, &self.scene, &self.queue)?;

        self.ui.render(
            &mut encoder,
            &output,
            &self.window,
            &self.device,
            &self.queue,
        );

        self.queue.submit(Some(encoder.finish()));
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

    fn handle_pointer_move(&mut self, position: PhysicalPosition<f64>) {
        let ray = self
            .scene
            .camera
            .screen_pos_to_ray(position, self.window_size);
        self.cursor_ray = ray;
    }

    fn handle_pointer_input(&mut self, button: MouseButton, state: ElementState) {
        if button == MouseButton::Left && state == ElementState::Pressed {
            let closest_hit = self
                .scene
                .hit_closest_sphere(&self.cursor_ray, 0.001, 1000.0);

            if let Some(HitRecord { sphere, .. }) = closest_hit {
                if sphere.material == Material::Gizmo {
                    return;
                }

                let mut gizmo = Sphere::new(SphereDescriptor {
                    center: sphere.center,
                    radius: sphere.radius + 0.01,
                    albedo: Vector3::new(1.0, 0.6, 0.0),
                    material: Material::Gizmo,
                });
                gizmo.label = Some("selected_sphere_gizmo".to_string());

                self.scene.selected_sphere = Some(sphere.uuid);
                self.scene
                    .spheres
                    .retain(|s| s.label != Some("selected_sphere_gizmo".to_string()));
                self.scene.spheres.push(gizmo);
            } else {
                self.scene.selected_sphere = None;
                self.scene
                    .spheres
                    .retain(|s| s.label != Some("selected_sphere_gizmo".to_string()));
            }
        }
    }

    pub fn input(&mut self, event: &Event<'_, ()>) {
        if self.ui.contains_mouse() {
            return;
        }

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if *window_id == self.window.id() => {
                self.camera_controller.input(event, &mut self.window);

                match event {
                    WindowEvent::Resized(physical_size) => {
                        self.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        self.resize(**new_inner_size);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        self.handle_pointer_move(*position);
                    }
                    WindowEvent::MouseInput { button, state, .. } => {
                        self.handle_pointer_input(*button, *state);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn run(mut self, event_loop: EventLoop<()>) {
        event_loop.run(move |event, _, control_flow| {
            self.ui_input(&event);

            self.input(&event);

            match event {
                Event::RedrawRequested(window_id) if window_id == self.window().id() => {
                    self.update();

                    match self.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            eprintln!("Lost surface, resizing");
                            self.resize(self.window_size());
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            eprintln!("Out of memory, exiting");
                            *control_flow = ControlFlow::Exit;
                        }
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                Event::MainEventsCleared => {
                    self.window().request_redraw();
                }

                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == self.window().id() => match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    _ => {}
                },
                _ => {}
            }
        });
    }
}
