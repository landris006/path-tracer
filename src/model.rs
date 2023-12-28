use std::{
    fs,
    io::{BufReader, Cursor},
};

use wgpu::{util::DeviceExt, Texture};

use crate::texture::Texture2D;

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    // pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub triangle_buffer: wgpu::Buffer,
    pub triangle_count: u32,
    pub material: usize,
}

impl Model {
    pub fn from_obj(
        file_path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        // layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, std::io::Error> {
        let obj_text = fs::read_to_string(file_path)?;
        let obj_cursor = Cursor::new(obj_text);
        let mut obj_reader = BufReader::new(obj_cursor);

        let (models, obj_materials) = tobj::load_obj_buf(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            |_| {
                let mat_text = fs::read_to_string(file_path).unwrap();
                tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
            },
        )
        .unwrap();

        let mut materials = Vec::new();
        for material in obj_materials.unwrap() {
            let diffuse_texture =
                Texture2D::from_file(&material.diffuse_texture.unwrap(), device, queue).unwrap();
            // let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            //     layout,
            //     entries: &[
            //         wgpu::BindGroupEntry {
            //             binding: 0,
            //             resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
            //         },
            //         wgpu::BindGroupEntry {
            //             binding: 1,
            //             resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
            //         },
            //     ],
            //     label: None,
            // });

            materials.push(Material {
                name: material.name,
                diffuse_texture: diffuse_texture.texture,
                // bind_group,
            })
        }

        let meshes = models
            .into_iter()
            .map(|model| {
                let triangles = model
                    .mesh
                    .indices
                    .chunks_exact(3)
                    .map(|c| Triangle {
                        a: [
                            model.mesh.positions[c[0] as usize * 3],
                            model.mesh.positions[c[0] as usize * 3 + 1],
                            model.mesh.positions[c[0] as usize * 3 + 2],
                        ],
                        b: [
                            model.mesh.positions[c[1] as usize * 3],
                            model.mesh.positions[c[1] as usize * 3 + 1],
                            model.mesh.positions[c[1] as usize * 3 + 2],
                        ],
                        c: [
                            model.mesh.positions[c[2] as usize * 3],
                            model.mesh.positions[c[2] as usize * 3 + 1],
                            model.mesh.positions[c[2] as usize * 3 + 2],
                        ],
                        na: [
                            model.mesh.normals[c[0] as usize * 3],
                            model.mesh.normals[c[0] as usize * 3 + 1],
                            model.mesh.normals[c[0] as usize * 3 + 2],
                        ],
                        nb: [
                            model.mesh.normals[c[1] as usize * 3],
                            model.mesh.normals[c[1] as usize * 3 + 1],
                            model.mesh.normals[c[1] as usize * 3 + 2],
                        ],
                        nc: [
                            model.mesh.normals[c[2] as usize * 3],
                            model.mesh.normals[c[2] as usize * 3 + 1],
                            model.mesh.normals[c[2] as usize * 3 + 2],
                        ],
                        ..Default::default()
                    })
                    .collect::<Vec<_>>();

                let triangle_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{:?} Triangle Buffer", file_path)),
                        contents: bytemuck::cast_slice(&triangles),
                        usage: wgpu::BufferUsages::STORAGE,
                    });

                dbg!(triangles.len());

                Mesh {
                    name: file_path.to_string(),
                    triangle_buffer,
                    triangle_count: triangles.len() as u32,
                    material: model.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<_>>();

        Ok(Model { meshes, materials })
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Triangle {
    a: [f32; 3],
    _pad0: f32,
    b: [f32; 3],
    _pad1: f32,
    c: [f32; 3],
    _pad2: f32,
    na: [f32; 3],
    _pad3: f32,
    nb: [f32; 3],
    _pad4: f32,
    nc: [f32; 3],
    _pad5: f32,
}

impl Default for Triangle {
    fn default() -> Self {
        Self {
            a: [0.0; 3],
            _pad0: 0.0,
            b: [0.0; 3],
            _pad1: 0.0,
            c: [0.0; 3],
            _pad2: 0.0,
            na: [0.0; 3],
            _pad3: 0.0,
            nb: [0.0; 3],
            _pad4: 0.0,
            nc: [0.0; 3],
            _pad5: 0.0,
        }
    }
}

