mod assets;
mod camera;
mod event;
mod graphics;
mod window;

use std::time::{Duration, Instant};

use crate::{camera::Camera, window::Window};
use event::Event;
use glam::{Quat, Vec3};
use graphics::Renderer;
use tecs::impl_archetype;
use thanatos_macros::Archetype;
use window::{Keyboard, Mouse};

/*
#[derive(Archetype)]
struct CopperOre {
    render: RenderObject,
}

#[derive(Archetype)]
struct Tree {
    render: RenderObject,
}
*/

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
async fn main() {
    pretty_env_logger::init();

    let window = Window::new();

    let renderer = Renderer::new(&window).unwrap();
    let camera = Camera::new(&window);

    /*
    let mut assets = assets::Manager::new();
    let copper_ore = assets.add_mesh(Mesh::load("assets/meshes/copper_ore.glb", &ctx.device));
    let tree = assets.add_mesh(Mesh::load("assets/meshes/tree.glb", &ctx.device));
    let material = assets.add_material(Material::load(
        MaterialData {
            colour: Vec4::X + Vec4::W,
        },
        &ctx.device,
        &ctx.material_bind_group_layout,
    ));
    */

    let mut world = World::new()
        .with_resource(State::Running)
        .with_resource(window)
        .with_resource(renderer)
        .with_resource(camera)
        //.with_resource(assets)
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
            let mut camera = world.get_mut::<Camera>().unwrap();
            let keyboard = world.get::<Keyboard>().unwrap();
            let mouse = world.get::<Mouse>().unwrap();
            let direction_xz = camera.direction * (Vec3::X + Vec3::Z).normalize();
            let rotation = Quat::from_rotation_arc(Vec3::Z, direction_xz);
            if keyboard.is_down("w") {
                camera.eye += rotation * Vec3::Z * 0.01
            }
            if keyboard.is_down("s") {
                camera.eye -= rotation * Vec3::Z * 0.01
            }
            if keyboard.is_down("a") {
                camera.eye += rotation * Vec3::X * 0.01
            }
            if keyboard.is_down("d") {
                camera.eye -= rotation * Vec3::X * 0.01
            }
            let rotation = Quat::from_rotation_y(-mouse.delta.x * 0.005);
            camera.direction = rotation * camera.direction;
            let rotation = Quat::from_rotation_x(-mouse.delta.y * 0.01);
            camera.direction = rotation * camera.direction;
        })
        .with_ticker(|world| {
            let clock = world.get::<Clock>().unwrap();
            //println!("FPS: {}", 1.0 / clock.frame_delta.as_secs_f32());
        })
        .with_ticker(Clock::tick)
        .with_handler(|world, event| match event {
            Event::Stop => {
                *world.get_mut::<State>().unwrap() = State::Stopped;
            }
            _ => (),
        });

    /*
    world.spawn(CopperOre {
        render: RenderObject {
            mesh: copper_ore,
            material,
        },
    });
    world.spawn(Tree {
        render: RenderObject {
            mesh: tree,
            material,
        },
    });
    */

    loop {
        if let State::Stopped = *world.get::<State>().unwrap() {
            break;
        }
        world.tick();
    }

    let renderer = world.take::<Renderer>().unwrap();
    renderer.destroy();
}
