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
struct CopperOre {
    render: RenderObject,
}

#[derive(Archetype)]
struct Tree {
    render: RenderObject
}

#[tokio::main]
async fn main() {
    let window = Window::new();
    let ctx = Context::new(&window).await;
    let camera = Camera::new(&window);

    let mut assets = assets::Manager::new();
    let copper_ore = assets.add_mesh(Mesh::load("assets/meshes/copper_ore.glb", &ctx.device));
    let tree = assets.add_mesh(Mesh::load("assets/meshes/tree.glb", &ctx.device));

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
            let eye = camera.eye;
            let direction = camera.direction;
            camera.eye += (eye - direction) * 0.001;
        })
        .register::<CopperOre>()
        .register::<Tree>();

    world.spawn(CopperOre {
        render: RenderObject { mesh: copper_ore },
    });
    world.spawn(Tree {
        render: RenderObject { mesh: tree },
    });
    world.run();
}
