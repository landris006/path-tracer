use std::{cell::RefCell, num::NonZeroU32, path::Path, time::Instant};

use cgmath::Vector3;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use wgpu::{
    util::DeviceExt, Buffer, BufferDescriptor, Device, Extent3d, Queue, SamplerBindingType,
    SurfaceConfiguration, SurfaceTexture, Texture, TextureFormat, TextureViewDescriptor,
};
use winit::{event::Event, window::Window};

use crate::{
    camera::{Camera, CameraBuffer},
    scene::{Material, Scene, Sphere, SphereBuffer},
    texture, CustomEvent, WINDOW_HEIGHT, WINDOW_WIDTH,
};

const NUMBER_OF_SAMPLES: usize = 16;

pub struct RendererDescriptor<'a> {
    pub window: &'a Window,
    pub surface_format: &'a TextureFormat,
    pub surface_config: &'a SurfaceConfiguration,
    pub device: &'a Device,
    pub queue: &'a Queue,
}

pub struct Renderer {
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,

    copy_pipeline: wgpu::RenderPipeline,
    copy_bind_group: wgpu::BindGroup,

    platform: RefCell<Platform>,
    egui_rpass: RenderPass,

    start_time: Instant,
    last_frame_time: std::time::Instant,
    frame_times: Vec<u128>,

    time_buffer: wgpu::Buffer,
    camera_buffer: Buffer,
    sphere_buffer: Buffer,

    progressive_rendering_samples: Option<u32>,
    output_textures: [Texture; NUMBER_OF_SAMPLES],
}

impl Renderer {
    pub fn new(
        RendererDescriptor {
            window,
            surface_format,
            surface_config,
            device,
            queue,
        }: RendererDescriptor,
    ) -> Self {
        let window_size = window.inner_size();

        let platform = Platform::new(PlatformDescriptor {
            physical_width: window_size.width,
            physical_height: window_size.height,
            scale_factor: window.scale_factor(),
            ..Default::default()
        });

        // We use the egui_wgpu_backend crate as the render backend.
        let egui_rpass = RenderPass::new(device, *surface_format, 1);

        let src = load_shader_source(Path::new("shaders"), "compute.wgsl").unwrap();
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute"),
            source: wgpu::ShaderSource::Wgsl(src.into()),
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // Output texture
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
                    // Camera
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
                    // Spheres
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
                    // Time
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
                    // Skydome texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
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
            texture::load_hdr_texture("assets/skydome.hdr", device, queue).unwrap();

        let sphere_buffer = device.create_buffer(&BufferDescriptor {
            mapped_at_creation: false,
            // TODO: memory
            size: std::mem::size_of::<SphereBuffer>() as u64 * 6,
            label: None,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let time_buffer = device.create_buffer(&BufferDescriptor {
            mapped_at_creation: false,
            size: std::mem::size_of::<u128>() as u64,
            label: None,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_buffer = device.create_buffer(&BufferDescriptor {
            mapped_at_creation: false,
            size: std::mem::size_of::<CameraBuffer>() as u64,
            label: None,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sky_texture_view = skydome_texture.create_view(&TextureViewDescriptor::default());

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
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&sky_texture_view),
                },
            ],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&compute_bind_group_layout],
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
                targets: &[Some(surface_config.format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Renderer {
            progressive_rendering_samples: None,
            output_textures,
            compute_pipeline,
            compute_bind_group,
            copy_pipeline,
            copy_bind_group,
            egui_rpass,
            platform: RefCell::new(platform),
            camera_buffer,
            time_buffer,
            start_time: Instant::now(),
            last_frame_time: Instant::now(),
            frame_times: Vec::new(),
            sphere_buffer,
        }
    }

    fn setup_ui(&mut self) {
        let mut platform = self.platform.borrow_mut();
        platform.update_time(self.start_time.elapsed().as_secs_f64());
        platform.begin_frame();

        let avg_frame_time = self.frame_times.iter().sum::<u128>() / self.frame_times.len() as u128;

        egui::Window::new("Settings")
            .resizable(true)
            .show(&platform.context(), |ui| {
                ui.add(egui::Label::new("hello"));
                ui.add(egui::Label::new(format!(
                    "Frame time: {}ms ({} FPS)",
                    avg_frame_time,
                    1000 / avg_frame_time
                )));
                // ui.add(
                //     egui::Slider::new(&mut self.camera_controller.speed, 0.0..=5.0)
                //         .text("Camera speed"),
                // );
                // ui.add(egui::Slider::new(&mut self.camera.vfov, 0.10..=100.0).text("Camera vfov"));
            });
    }

    fn update_time(&mut self) {
        let now = Instant::now();
        let delta = now - self.last_frame_time;
        self.last_frame_time = now;

        self.frame_times.push(delta.as_millis());
        if self.frame_times.len() > 100 {
            self.frame_times.remove(0);
        }
    }

    fn update_buffers(&mut self, queue: &Queue, scene: &Scene) {
        queue.write_buffer(
            &self.time_buffer,
            0,
            bytemuck::cast_slice(&[self.start_time.elapsed().as_millis() / 4]),
        );

        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[CameraBuffer::from(&scene.camera)]),
        );

        queue.write_buffer(
            &self.sphere_buffer,
            0,
            bytemuck::cast_slice(
                &scene
                    .spheres
                    .iter()
                    .map(SphereBuffer::from)
                    .collect::<Vec<_>>(),
            ),
        );
    }

    pub fn render(
        &mut self,
        output: &mut SurfaceTexture,
        scene: &Scene,
        device: &Device,
        queue: &Queue,
        window: &Window,
    ) -> Result<(), wgpu::SurfaceError> {
        self.update_time();
        self.update_buffers(queue, scene);
        self.setup_ui();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
        compute_pass.dispatch_workgroups(
            output.texture.width() / 16,
            output.texture.height() / 16,
            1,
        );
        drop(compute_pass);

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
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

        let full_output = platform.end_frame(Some(window));
        let paint_jobs = platform.context().tessellate(full_output.shapes);

        let screen_descriptor = ScreenDescriptor {
            physical_width: output.texture.width(),
            physical_height: output.texture.height(),
            scale_factor: window.scale_factor() as f32,
        };

        self.egui_rpass
            .add_textures(device, queue, &full_output.textures_delta)
            .expect("error adding textures");
        self.egui_rpass
            .update_buffers(device, queue, &paint_jobs, &screen_descriptor);
        self.egui_rpass
            .execute(&mut encoder, &view, &paint_jobs, &screen_descriptor, None)
            .unwrap();

        queue.submit(Some(encoder.finish()));

        self.egui_rpass
            .remove_textures(full_output.textures_delta)
            .expect("error removing textures");

        Ok(())
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
