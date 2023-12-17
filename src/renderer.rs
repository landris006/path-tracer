use std::{num::NonZeroU32, path::Path, time::Instant};

use egui_winit_platform::Platform;
use wgpu::{
    Buffer, BufferDescriptor, CommandEncoder, Device, Extent3d, Queue, SamplerBindingType,
    SurfaceConfiguration, SurfaceTexture, Texture, TextureViewDescriptor,
};

use crate::{
    camera::CameraBuffer,
    scene::{Scene, SphereBuffer},
    texture, WINDOW_HEIGHT, WINDOW_WIDTH,
};

const MAX_NUMBER_OF_SAMPLES: u32 = 256;

struct ProgressiveRendering {
    sample_size: u32,
    sample_size_while_moving: u32,
    buffer: Buffer,
    ready_samples: u32,
    output_textures: [Texture; MAX_NUMBER_OF_SAMPLES as usize],
}

impl ProgressiveRendering {
    fn get_sample_size(&self, is_moving: bool) -> u32 {
        if is_moving {
            self.sample_size_while_moving
        } else {
            u32::min(self.sample_size, self.ready_samples)
        }
    }

    fn increment_ready_samples(&mut self) {
        self.ready_samples = u32::min(self.ready_samples + 1, self.sample_size);
    }
}

pub struct Renderer {
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,

    copy_pipeline: wgpu::RenderPipeline,
    copy_bind_group: wgpu::BindGroup,

    start_time: Instant,

    time_buffer: wgpu::Buffer,
    camera_buffer: Buffer,
    sphere_buffer: Buffer,

    progressive_rendering: ProgressiveRendering,
}

impl Renderer {
    pub fn new(device: &Device, queue: &Queue, surface_config: &SurfaceConfiguration) -> Self {
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

        let output_textures: [Texture; MAX_NUMBER_OF_SAMPLES as usize] = (0..MAX_NUMBER_OF_SAMPLES)
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
                    // Output textures
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: Some(NonZeroU32::new(MAX_NUMBER_OF_SAMPLES).unwrap()),
                    },
                    // Output texture sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    // Progressive rendering samples
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
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

        let progressive_rendering_samples_buffer = device.create_buffer(&BufferDescriptor {
            mapped_at_creation: false,
            size: std::mem::size_of::<u32>() as u64,
            label: None,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let copy_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &copy_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(
                        (0..MAX_NUMBER_OF_SAMPLES)
                            .map(|i| &views[i as usize])
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: progressive_rendering_samples_buffer.as_entire_binding(),
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
            progressive_rendering: ProgressiveRendering {
                sample_size: MAX_NUMBER_OF_SAMPLES,
                sample_size_while_moving: 4,
                ready_samples: 0,
                buffer: progressive_rendering_samples_buffer,
                output_textures,
            },
            compute_pipeline,
            compute_bind_group,
            copy_pipeline,
            copy_bind_group,
            camera_buffer,
            time_buffer,
            start_time: Instant::now(),
            sphere_buffer,
        }
    }

    pub fn render_ui(&mut self, platform: &mut Platform, is_moving: bool) {
        egui::Window::new("Renderer settings")
            .resizable(true)
            .show(&platform.context(), |ui| {
                ui.add(egui::Label::new("Progressive rendering"));
                ui.add(
                    egui::Slider::new(
                        &mut self.progressive_rendering.sample_size,
                        1..=MAX_NUMBER_OF_SAMPLES,
                    )
                    .text("samples"),
                );

                ui.add(egui::Label::new(format!(
                    "Samples used: {}/{}",
                    self.progressive_rendering.get_sample_size(is_moving),
                    MAX_NUMBER_OF_SAMPLES
                )));

                ui.add(
                    egui::Slider::new(
                        &mut self.progressive_rendering.sample_size_while_moving,
                        1..=MAX_NUMBER_OF_SAMPLES,
                    )
                    .text("samples while moving"),
                );
            });
    }

    fn update(&mut self, scene: &Scene) {
        if scene.camera.moved_recently() {
            self.progressive_rendering.ready_samples = 0;
        }
    }

    fn update_buffers(&mut self, queue: &Queue, encoder: &mut CommandEncoder, scene: &Scene) {
        (1..self
            .progressive_rendering
            .get_sample_size(scene.camera.moved_recently()))
            .rev()
            .for_each(|i| {
                encoder.copy_texture_to_texture(
                    wgpu::ImageCopyTexture {
                        texture: &self.progressive_rendering.output_textures[(i - 1) as usize],
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::ImageCopyTexture {
                        texture: &self.progressive_rendering.output_textures[i as usize],
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

        queue.write_buffer(
            &self.progressive_rendering.buffer,
            0,
            bytemuck::cast_slice(&[self
                .progressive_rendering
                .get_sample_size(scene.camera.moved_recently())]),
        );
    }

    pub fn render(
        &mut self,
        output: &mut SurfaceTexture,
        encoder: &mut CommandEncoder,
        scene: &Scene,
        queue: &Queue,
    ) -> Result<(), wgpu::SurfaceError> {
        self.update(scene);
        self.update_buffers(queue, encoder, scene);
        self.progressive_rendering.increment_ready_samples();

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

        Ok(())
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
