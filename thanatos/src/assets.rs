use std::path::Path;

use anyhow::Result;
use glam::{Vec3, Vec4};
use gltf::Glb;
use hephaestus::{buffer::Static, BufferUsageFlags, Context, VkResult};

use crate::graphics::{Renderer, Vertex};

pub struct Mesh {
    pub vertex_buffer: Static,
    pub index_buffer: Static,
    pub num_indices: u32,
}

impl Mesh {
    pub fn load<T: AsRef<Path>>(path: T, renderer: &Renderer) -> Result<Self> {
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

        let vertex_buffer = Static::new(
            &renderer.ctx,
            bytemuck::cast_slice::<Vertex, u8>(&vertices),
            BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let index_buffer = Static::new(
            &renderer.ctx,
            bytemuck::cast_slice::<u32, u8>(&indices),
            BufferUsageFlags::INDEX_BUFFER,
        )?;

        Ok(Mesh {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialData {
    pub colour: Vec4,
}

pub struct Material {
    pub buffer: Static,
}

impl Material {
    pub fn load(material: MaterialData, renderer: &Renderer) -> Result<Self> {
        let contents = bytemuck::bytes_of(&material);
        let buffer = Static::new(&renderer.ctx, &contents, BufferUsageFlags::UNIFORM_BUFFER)?;
        Ok(Self { buffer })
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
