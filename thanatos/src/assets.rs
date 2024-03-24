use wgpu::Buffer;

pub struct Mesh {
    pub vertices: Buffer,
    pub indices: Buffer,
    pub num_indices: u32
}
#[derive(Clone, Copy, Debug)]
pub struct MeshId(usize);

#[derive(Default)]
pub struct Manager {
    meshes: Vec<Mesh>,
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
}
