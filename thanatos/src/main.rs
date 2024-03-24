mod assets;
mod camera;
mod event;
mod graphics;
mod structures;
mod window;
mod world;

use assets::MeshId;
use glam::{Quat, Vec3};
use graphics::Context;
use world::impl_archetype;

use crate::{camera::Camera, window::Window, world::World};

impl_archetype!(Player, PlayerRef, PlayerMut, position: Vec3);

#[tokio::main]
async fn main() {
    let window = Window::new();
    let ctx = Context::new(&window).await;
    let camera = Camera::new(&window);
    let mut world = World::new()
        .with_resource(window)
        .with_resource(ctx)
        .with_resource(camera)
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
    });
    world.run();
}
