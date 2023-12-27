use std::{io::Cursor, path::Path};

use crate::utils;
use image::{
    codecs::hdr::{HdrDecoder, HdrMetadata},
    ImageResult,
};
use wgpu::{Sampler, SamplerDescriptor, Texture, TextureView, TextureViewDescriptor};

pub struct CubeTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

impl CubeTexture {
    fn create_2d(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        mip_level_count: u32,
        usage: wgpu::TextureUsages,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Cube Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 6,
            },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("Sky texture view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            array_layer_count: Some(6),
            ..Default::default()
        });
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            sampler,
            view,
        }
    }

    pub fn from_equirectangular_hdri(
        hdr_loader: &HdrLoader,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        dst_size: u32,
    ) -> ImageResult<Self> {
        let hdr_decoder = HdrDecoder::new(Cursor::new(data))?;
        let HdrMetadata { width, height, .. } = hdr_decoder.metadata();
        let mut pixels = vec![[0.0, 0.0, 0.0, 0.0]; width as usize * height as usize];
        hdr_decoder.read_image_transform(
            |pix| {
                let rgb = pix.to_hdr();
                [rgb.0[0], rgb.0[1], rgb.0[2], 1.0f32]
            },
            &mut pixels[..],
        )?;

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let src = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("HDR Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &src,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&pixels),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width * std::mem::size_of::<[f32; 4]>() as u32),
                rows_per_image: Some(height),
            },
            size,
        );

        let src_view = src.create_view(&TextureViewDescriptor {
            label: Some("HDR Texture view"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            ..Default::default()
        });

        let dst = CubeTexture::create_2d(
            device,
            dst_size,
            dst_size,
            hdr_loader.texture_format,
            1,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        );

        let dst_view = dst.texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("equirect_to_cubemap bind group"),
            layout: &hdr_loader.equirect_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&dst_view),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&Default::default());
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());

        let num_workgroups = (dst_size + 15) / 16;
        pass.set_pipeline(&hdr_loader.equirect_to_cubemap);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(num_workgroups, num_workgroups, 6);

        drop(pass);

        queue.submit([encoder.finish()]);

        Ok(dst)
    }
}

pub struct HdrLoader {
    texture_format: wgpu::TextureFormat,
    equirect_layout: wgpu::BindGroupLayout,
    equirect_to_cubemap: wgpu::ComputePipeline,
}

impl HdrLoader {
    pub fn new(device: &wgpu::Device) -> Self {
        let src = utils::load_shader_source(Path::new("shaders"), "equirectangular.wgsl").unwrap();

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute"),
            source: wgpu::ShaderSource::Wgsl(src.into()),
        });
        let texture_format = wgpu::TextureFormat::Rgba32Float;
        let equirect_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("HdrLoader::equirect_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: texture_format,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&equirect_layout],
            push_constant_ranges: &[],
        });

        let equirect_to_cubemap =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("equirect_to_cubemap"),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: "compute_equirect_to_cubemap",
            });

        Self {
            equirect_to_cubemap,
            texture_format,
            equirect_layout,
        }
    }
}
