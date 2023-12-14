use std::{
    cell::RefCell,
    num::{self, NonZeroU32},
    path::Path,
    time::Instant,
};

use cgmath::Vector3;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use wgpu::{util::DeviceExt, Extent3d, SamplerBindingType, Texture, TextureViewDescriptor};
use winit::{
    event::{Event, WindowEvent},
    window::Window,
};

use crate::{
    camera::{Camera, CameraBuffer, CameraController},
    scene::{Material, Sphere, SphereBuffer},
    texture, CustomEvent, WINDOW_HEIGHT, WINDOW_WIDTH,
};

const NUMBER_OF_SAMPLES: usize = 16;

pub struct App {
    start_time: Instant,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window_size: winit::dpi::PhysicalSize<u32>,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    sky_dome_texture_bind_group: wgpu::BindGroup,
    copy_pipeline: wgpu::RenderPipeline,
    copy_bind_group: wgpu::BindGroup,
    time_buffer: wgpu::Buffer,
    last_frame_time: std::time::Instant,
    camera: Camera,
    camera_controller: CameraController,
    camera_buffer: wgpu::Buffer,
    platform: RefCell<Platform>,
    egui_rpass: RenderPass,
    output_textures: [Texture; NUMBER_OF_SAMPLES],
    frame_times: Vec<f32>,
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
                        wgpu::Limits::default()
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

        let platform = Platform::new(PlatformDescriptor {
            physical_width: window_size.width,
            physical_height: window_size.height,
            scale_factor: window.scale_factor(),
            ..Default::default()
        });

        // We use the egui_wgpu_backend crate as the render backend.
        let egui_rpass = RenderPass::new(&device, surface_format, 1);

        let src = load_shader_source(Path::new("shaders"), "compute.wgsl").unwrap();
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute"),
            source: wgpu::ShaderSource::Wgsl(src.into()),
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let output_textures: [Texture; NUMBER_OF_SAMPLES] = (0..NUMBER_OF_SAMPLES)
            .map(|_| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        width: WINDOW_WIDTH,
                        height: WINDOW_HEIGHT,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::STORAGE_BINDING
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                })
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let views = output_textures
            .iter()
            .map(|texture| texture.create_view(&TextureViewDescriptor::default()))
            .collect::<Vec<_>>();

        let skydome_texture =
            texture::load_hdr_texture("assets/skydome.hdr", &device, &queue).unwrap();

        let camera = Camera {
            origin: Vector3::new(0.0, 0.0, 0.0),
            forward: Vector3::new(0.0, 0.0, -1.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            right: Vector3::new(1.0, 0.0, 0.0),
            focal_length: 1.0,
            vfov: 90.0,
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
        let sphere_buffer = spheres.iter().map(SphereBuffer::from).collect::<Vec<_>>();

        let sphere_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&sphere_buffer),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[CameraBuffer::from(&camera)]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            // TODO:
            contents: bytemuck::cast_slice(&[Instant::now().elapsed().as_millis()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(views.first().unwrap()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: sphere_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: time_buffer.as_entire_binding(),
                },
            ],
        });

        let sky_dome_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let sky_texture_view = skydome_texture.create_view(&TextureViewDescriptor::default());

        let sky_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let sky_dome_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &sky_dome_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&sky_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sky_sampler),
                },
            ],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    &compute_bind_group_layout,
                    &sky_dome_texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        let src = load_shader_source(Path::new("shaders"), "copy.wgsl").unwrap();
        let copy_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("copy"),
            source: wgpu::ShaderSource::Wgsl(src.into()),
        });

        let copy_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: Some(NonZeroU32::new(NUMBER_OF_SAMPLES as u32).unwrap()),
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let copy_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &copy_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(
                        (0..NUMBER_OF_SAMPLES)
                            .map(|i| &views[i])
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let copy_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Copy Pipeline Layout"),
            bind_group_layouts: &[&copy_bind_group_layout],
            push_constant_ranges: &[],
        });
        let copy_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Copy Pipeline"),
            layout: Some(&copy_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &copy_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &copy_shader,
                entry_point: "fs_main",
                targets: &[Some(config.format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            window_size,
            copy_pipeline,
            copy_bind_group,
            compute_pipeline,
            compute_bind_group,
            sky_dome_texture_bind_group,
            last_frame_time: Instant::now(),
            time_buffer,
            start_time: Instant::now(),
            camera,
            camera_controller: CameraController::new(),
            camera_buffer,
            platform: RefCell::new(platform),
            egui_rpass,
            output_textures,
            frame_times: Vec::new(),
            window,
        }
    }

    pub fn update(&mut self) {
        let now = std::time::Instant::now();
        let delta = now - self.last_frame_time;
        self.last_frame_time = now;
        self.frame_times.push(delta.as_secs_f32());
        if self.frame_times.len() > 100 {
            self.frame_times.remove(0);
        }

        self.camera_controller
            .update_camera(&mut self.camera, delta.as_secs_f32());

        self.queue.write_buffer(
            &self.time_buffer,
            0,
            bytemuck::cast_slice(&[self.start_time.elapsed().as_millis() / 4]),
        );
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[CameraBuffer::from(&self.camera)]),
        );
    }

    pub fn setup_ui(&mut self) {
        let mut platform = self.platform.borrow_mut();
        platform.update_time(self.start_time.elapsed().as_secs_f64());
        platform.begin_frame();

        let avg_frame_time = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;

        egui::Window::new("Settings")
            .resizable(true)
            .show(&platform.context(), |ui| {
                ui.add(egui::Label::new(format!("FPS: {}", 1.0 / avg_frame_time)));
                ui.add(
                    egui::Slider::new(&mut self.camera_controller.speed, 0.0..=5.0)
                        .text("Camera speed"),
                );
                ui.add(egui::Slider::new(&mut self.camera.vfov, 0.10..=100.0).text("Camera vfov"));
            });
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        (1..NUMBER_OF_SAMPLES).rev().for_each(|i| {
            encoder.copy_texture_to_texture(
                wgpu::ImageCopyTexture {
                    texture: &self.output_textures[i - 1],
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyTexture {
                    texture: &self.output_textures[i],
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                Extent3d {
                    width: WINDOW_WIDTH,
                    height: WINDOW_HEIGHT,
                    depth_or_array_layers: 1,
                },
            );
        });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
        compute_pass.set_bind_group(1, &self.sky_dome_texture_bind_group, &[]);
        compute_pass.dispatch_workgroups(
            self.window_size.width / 16,
            self.window_size.height / 16,
            1,
        );
        drop(compute_pass);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_bind_group(0, &self.copy_bind_group, &[]);
        render_pass.set_pipeline(&self.copy_pipeline);
        render_pass.draw(0..3, 0..2);

        drop(render_pass);

        let mut platform = self.platform.borrow_mut();

        let full_output = platform.end_frame(Some(self.window()));
        let paint_jobs = platform.context().tessellate(full_output.shapes);

        let screen_descriptor = ScreenDescriptor {
            physical_width: self.window_size.width,
            physical_height: self.window_size.height,
            scale_factor: self.window.scale_factor() as f32,
        };

        self.egui_rpass
            .add_textures(&self.device, &self.queue, &full_output.textures_delta)
            .expect("error adding textures");
        self.egui_rpass
            .update_buffers(&self.device, &self.queue, &paint_jobs, &screen_descriptor);
        self.egui_rpass
            .execute(&mut encoder, &view, &paint_jobs, &screen_descriptor, None)
            .unwrap();

        self.queue.submit(Some(encoder.finish()));
        output.present();

        self.egui_rpass
            .remove_textures(full_output.textures_delta)
            .expect("error removing textures");

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

    pub fn ui_input(&mut self, event: &Event<CustomEvent>) {
        self.platform.borrow_mut().handle_event(event);
    }
}

fn load_shader_source(shaders_root: &Path, name: &str) -> Result<String, std::io::Error> {
    let path = std::path::Path::new(shaders_root).join(name);
    let src = std::fs::read_to_string(path)?
        .lines()
        .map(|line| {
            if line.starts_with("//!include") {
                let path = line
                    .split_whitespace()
                    .nth(1)
                    .expect("invalid include statement")
                    .replace('"', "");
                load_shader_source(&Path::new(shaders_root).join("include"), &path)
            } else {
                Ok(line.to_owned())
            }
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");

    Ok(src)
}
