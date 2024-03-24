mod assets;
mod camera;
mod event;
mod graphics;
mod structures;
mod window;
mod world;

use crate::{camera::Camera, window::Window, world::World};
use assets::Mesh;
use glam::{Quat, Vec3};
use gltf::Glb;
use graphics::{Context, RenderObject, Vertex};
use thanatos_macros::Archetype;
use wgpu::util::DeviceExt;
use world::impl_archetype;

#[derive(Archetype)]
struct Player {
    render: RenderObject,
}

#[tokio::main]
async fn main() {
    let window = Window::new();
    let ctx = Context::new(&window).await;
    let camera = Camera::new(&window);

    let model = Glb::load(&std::fs::read("assets/meshes/copper_ore.glb").unwrap()).unwrap();

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
        .map(|(position, normal)| Vertex { position, normal, colour: Vec3::ONE })
        .collect();

    let indices: Vec<u32> = model.gltf.meshes[0].primitives[0]
        .get_indices_data(&model)
        .unwrap();

    let vertex_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

    let index_buffer = ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

    let mesh = Mesh {
        vertex_buffer,
        index_buffer,
        num_indices: indices.len() as u32,
    };
    let mut assets = assets::Manager::new();
    let mesh = assets.add_mesh(mesh);

    let mut world = World::new()
        .with_resource(window)
        .with_resource(ctx)
        .with_resource(camera)
        .with_resource(assets)
        .with_ticker(window::poll_events)
        .with_handler(camera::handle_resize)
        .with_handler(graphics::resize_surface)
        .with_ticker(graphics::draw)
        .with_ticker(|world| {
            let mut camera = world.get_mut::<Camera>().unwrap();
            camera.eye = Quat::from_rotation_y(0.01).mul_vec3(camera.eye);
            camera.direction = -camera.eye;
        })
        .register::<Player>();

    world.spawn(Player {
        render: RenderObject { mesh },
    });
    world.run();
}
