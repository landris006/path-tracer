use std::{
    fs,
    io::{BufReader, Cursor},
};

use cgmath::Vector3;
use wgpu::Texture;

use crate::{scene::Material, texture::Texture2D};

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<DiffuseTexture>,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
}

#[derive(Debug)]
pub struct Triangle {
    pub a: Vector3<f32>,
    pub b: Vector3<f32>,
    pub c: Vector3<f32>,
    pub na: Vector3<f32>,
    pub nb: Vector3<f32>,
    pub nc: Vector3<f32>,
    pub albedo: Vector3<f32>,
    pub material: Material,
}

impl Triangle {
    pub fn vertices(&self) -> [Vector3<f32>; 3] {
        [self.a, self.b, self.c]
    }

    pub fn centroid(&self) -> [f32; 3] {
        [
            (self.a[0] + self.b[0] + self.c[0]) / 3.0,
            (self.a[1] + self.b[1] + self.c[1]) / 3.0,
            (self.a[2] + self.b[2] + self.c[2]) / 3.0,
        ]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TriangleBuffer {
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
    albedo: [f32; 3],
    material: u32,
}

impl From<&Triangle> for TriangleBuffer {
    fn from(triangle: &Triangle) -> Self {
        Self {
            a: triangle.a.into(),
            b: triangle.b.into(),
            c: triangle.c.into(),
            na: triangle.na.into(),
            nb: triangle.nb.into(),
            nc: triangle.nc.into(),
            albedo: triangle.albedo.into(),
            material: match triangle.material {
                Material::Diffuse => 0,
                Material::Metal => 1,
                Material::Dielectric => 2,
                Material::Gizmo => 3,
            },
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
            _pad4: 0.0,
            _pad5: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct DiffuseTexture {
    pub name: String,
    pub diffuse_texture: Texture,
    // pub bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub triangles: Vec<Triangle>,
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

            materials.push(DiffuseTexture {
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
                    .map(|chunk| Triangle {
                        a: Vector3::new(
                            model.mesh.positions[chunk[0] as usize * 3],
                            model.mesh.positions[chunk[0] as usize * 3 + 1],
                            model.mesh.positions[chunk[0] as usize * 3 + 2],
                        ),
                        b: Vector3::new(
                            model.mesh.positions[chunk[1] as usize * 3],
                            model.mesh.positions[chunk[1] as usize * 3 + 1],
                            model.mesh.positions[chunk[1] as usize * 3 + 2],
                        ),
                        c: Vector3::new(
                            model.mesh.positions[chunk[2] as usize * 3],
                            model.mesh.positions[chunk[2] as usize * 3 + 1],
                            model.mesh.positions[chunk[2] as usize * 3 + 2],
                        ),
                        na: Vector3::new(
                            model.mesh.normals[chunk[0] as usize * 3],
                            model.mesh.normals[chunk[0] as usize * 3 + 1],
                            model.mesh.normals[chunk[0] as usize * 3 + 2],
                        ),
                        nb: Vector3::new(
                            model.mesh.normals[chunk[1] as usize * 3],
                            model.mesh.normals[chunk[1] as usize * 3 + 1],
                            model.mesh.normals[chunk[1] as usize * 3 + 2],
                        ),
                        nc: Vector3::new(
                            model.mesh.normals[chunk[2] as usize * 3],
                            model.mesh.normals[chunk[2] as usize * 3 + 1],
                            model.mesh.normals[chunk[2] as usize * 3 + 2],
                        ),
                        albedo: Vector3::new(1.0, 1.0, 1.0),
                        material: Material::Diffuse,
                    })
                    .collect::<Vec<_>>();

                Mesh {
                    name: file_path.to_string(),
                    triangles,
                    material: model.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<_>>();

        Ok(Model { meshes, materials })
    }
}

