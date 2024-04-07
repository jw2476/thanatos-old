mod assets;
mod camera;
mod event;
mod graphics;
mod window;

use std::time::{Duration, Instant};

use crate::{camera::Camera, window::Window};
use anyhow::Result;
use assets::Mesh;
use event::Event;
use glam::{Quat, Vec3};
use graphics::{RenderObject, Renderer};
use tecs::impl_archetype;
use thanatos_macros::Archetype;
use window::{Keyboard, Mouse};

#[derive(Archetype)]
struct CopperOre {
    render: RenderObject,
}

#[derive(Archetype)]
struct Tree {
    render: RenderObject,
}

#[derive(Clone, Debug)]
pub struct Clock {
    frame_delta: Duration,
    start: Instant,
    last: Instant,
}

impl Clock {
    pub fn tick(world: &mut World) {
        let mut clock = world.get_mut::<Clock>().unwrap();
        let now = Instant::now();
        clock.frame_delta = now - clock.last;
        clock.last = now;
    }
}

#[derive(Copy, Clone, Debug)]
pub enum State {
    Stopped,
    Running,
}

pub type World = tecs::World<Event>;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let window = Window::new();

    let renderer = Renderer::new(&window)?;
    let camera = Camera::new(&window);

    let mut assets = assets::Manager::new();
    let copper_ore = assets.add_mesh(Mesh::load("assets/meshes/copper_ore.glb", &renderer)?);
    let tree = assets.add_mesh(Mesh::load("assets/meshes/tree.glb", &renderer)?);
    let mut world = World::new()
        .with_resource(State::Running)
        .with_resource(window)
        .with_resource(renderer)
        .with_resource(camera)
        .with_resource(assets)
        .with_resource(Mouse::default())
        .with_resource(Keyboard::default())
        .with_resource(Clock {
            frame_delta: Duration::default(),
            start: Instant::now(),
            last: Instant::now(),
        })
        .with_ticker(window::clear_mouse_delta)
        .with_ticker(window::poll_events)
        .with_handler(camera::handle_resize)
        .with_ticker(graphics::draw)
        .with_ticker(|world| {
            let clock = world.get::<Clock>().unwrap();
            println!("FPS: {}", 1.0 / clock.frame_delta.as_secs_f32());
        })
        .with_ticker(Clock::tick)
        .with_handler(|world, event| match event {
            Event::Stop => {
                *world.get_mut::<State>().unwrap() = State::Stopped;
            }
            _ => (),
        });

    world.spawn(CopperOre {
        render: RenderObject { mesh: copper_ore },
    });

    loop {
        if let State::Stopped = *world.get::<State>().unwrap() {
            break;
        }
        world.tick();
    }

    let renderer = world.remove::<Renderer>().unwrap();
    renderer.destroy();

    Ok(())
}
