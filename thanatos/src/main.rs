mod camera;
mod event;
mod graphics;
mod window;
mod world;

use std::time::Instant;

use crate::{camera::Camera, graphics::GraphicsSystem, window::Window, world::World};

#[tokio::main]
async fn main() {
    let window = Window::new();
    let ctx = graphics::Graphics::new(&window).await;
    let camera = Camera::new(&window);
    let mut world = World::new()
        .with_resource(window)
        .with_resource(ctx)
        .with_resource(camera)
        .with_ticker(window::poll_events)
        .with_system(GraphicsSystem {})
        .with_handler(camera::handle_resize)
        .with_ticker(|world| {
            let mut camera = world.get_mut::<Camera>().unwrap();
            camera.eye = glam::Quat::from_rotation_y(0.01).mul_vec3(camera.eye);
            camera.direction = -camera.eye;
        });
    world.run();
}
