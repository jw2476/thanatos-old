mod assets;
mod camera;
mod event;
mod graphics;
mod structures;
mod window;
mod world;

use assets::{Mesh, MeshId};
use glam::{Mat4, Quat, Vec3};
use graphics::{Context, RenderObject, Vertex};
use wgpu::util::DeviceExt;
use world::impl_archetype;

use crate::{camera::Camera, window::Window, world::World};

impl_archetype!(struct Player { 
    position: Vec3, 
    render: RenderObject 
});

#[tokio::main]
async fn main() {
    let window = Window::new();
    let ctx = Context::new(&window).await;
    let camera = Camera::new(&window);

    let vertices = [
        Vertex {
            position: Vec3::new(0.0, 0.5, 0.0),
            colour: Vec3::X,
        },
        Vertex {
            position: Vec3::new(-0.5, -0.5, 0.0),
            colour: Vec3::Y,
        },
        Vertex {
            position: Vec3::new(0.5, -0.5, 0.0),
            colour: Vec3::Z,
        },
    ];

    let indices = [0, 1, 2];

    let vertices = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let indices = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    let mesh = Mesh { vertices, indices, num_indices: 3 };
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
        .with_ticker(|world| {
            println!(
                "{:?}",
                world
                    .get_entities::<Player>()
                    .iter()
                    .map(|player| &player.position)
                    .collect::<Vec<_>>()
            )
        })
        .register::<Player>();

    world.spawn(Player {
        position: Vec3::ONE,
        render: RenderObject { mesh }
    });
    world.run();
}
