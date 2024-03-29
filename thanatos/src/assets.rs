use std::path::Path;

use glam::{Vec3, Vec4};
use gltf::Glb;
use wgpu::{util::DeviceExt, BindGroup, Buffer, BufferUsages};

use crate::graphics::Vertex;

pub struct Mesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub num_indices: u32,
}

impl Mesh {
    pub fn load<T: AsRef<Path>>(path: T, device: &wgpu::Device) -> Self {
        let model = Glb::load(&std::fs::read(path).unwrap()).unwrap();

        let positions: Vec<Vec3> = bytemuck::cast_slice::<u8, f32>(
            &model.gltf.meshes[0].primitives[0]
                .get_attribute_data(&model, "POSITION")
                .unwrap(),
        )
        .chunks(3)
        .map(|pos| Vec3::from_slice(pos))
        .collect();

        let normals: Vec<Vec3> = bytemuck::cast_slice::<u8, f32>(
            &model.gltf.meshes[0].primitives[0]
                .get_attribute_data(&model, "NORMAL")
                .unwrap(),
        )
        .chunks(3)
        .map(|pos| Vec3::from_slice(pos))
        .collect();

        let vertices: Vec<Vertex> = positions
            .into_iter()
            .zip(normals.into_iter())
            .map(|(position, normal)| Vertex { position, normal })
            .collect();

        let indices: Vec<u32> = model.gltf.meshes[0].primitives[0]
            .get_indices_data(&model)
            .unwrap();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Mesh {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialData {
    pub colour: Vec4,
}

pub struct Material {
    pub buffer: Buffer,
    pub bind_group: BindGroup,
}

impl Material {
    pub fn load(
        material: MaterialData,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let contents = bytemuck::bytes_of(&material);
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents,
            usage: BufferUsages::UNIFORM,
            label: None,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
        });
        Self { buffer, bind_group }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MeshId(usize);
#[derive(Clone, Copy, Debug)]
pub struct MaterialId(usize);

#[derive(Default)]
pub struct Manager {
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
}

impl Manager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_mesh(&mut self, mesh: Mesh) -> MeshId {
        self.meshes.push(mesh);
        MeshId(self.meshes.len() - 1)
    }

    pub fn get_mesh(&self, id: MeshId) -> Option<&Mesh> {
        self.meshes.get(id.0)
    }

    pub fn add_material(&mut self, material: Material) -> MaterialId {
        self.materials.push(material);
        MaterialId(self.materials.len() - 1)
    }

    pub fn get_material(&self, id: MaterialId) -> Option<&Material> {
        self.materials.get(id.0)
    }
}
