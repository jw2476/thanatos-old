mod assets;
mod camera;
mod event;
mod graphics;
mod window;

use crate::{camera::Camera, window::Window};
use assets::{Material, MaterialData, Mesh};
use event::Event;
use glam::{Quat, Vec3, Vec4};
use graphics::RenderObject;
use hephaestus::{
    pipeline::{
        self, Framebuffer, ImageLayout, PipelineBindPoint, RenderPass, ShaderModule, Subpass,
    },
    Context, VkResult,
};
use log::warn;
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
    let ctx = Context::new("thanatos", &window.window).unwrap();

    let vertex = ShaderModule::new(
        &ctx.device,
        &std::fs::read("assets/shaders/shader.vert.spv").unwrap(),
    )
    .unwrap();

    let fragment = ShaderModule::new(
        &ctx.device,
        &std::fs::read("assets/shaders/shader.frag.spv").unwrap(),
    )
    .unwrap();

    let render_pass = {
        let mut builder = RenderPass::builder();
        let attachment = builder.attachment(
            ctx.swapchain.format,
            ImageLayout::UNDEFINED,
            ImageLayout::PRESENT_SRC_KHR,
        );
        builder.subpass(
            Subpass::new(PipelineBindPoint::GRAPHICS)
                .colour(attachment, ImageLayout::COLOR_ATTACHMENT_OPTIMAL),
        );
        builder.build(&ctx.device).unwrap()
    };

    let pipeline = pipeline::Graphics::builder()
        .vertex(&vertex)
        .fragment(&fragment)
        .render_pass(&render_pass)
        .subpass(0)
        .viewport(
            ctx.swapchain.extent.width as f32,
            ctx.swapchain.extent.height as f32,
        )
        .build(&ctx.device)
        .unwrap();

    let framebuffers = ctx
        .swapchain
        .views
        .iter()
        .map(|view| render_pass.get_framebuffer(&ctx.device, &[view]))
        .collect::<VkResult<Vec<Framebuffer>>>()
        .unwrap();

    framebuffers
        .into_iter()
        .for_each(|framebuffer| framebuffer.destroy(&ctx.device));

    pipeline.destroy(&ctx.device);
    render_pass.destroy(&ctx.device);
    vertex.destroy(&ctx.device);
    fragment.destroy(&ctx.device);
    ctx.destroy();

    /*
        let ctx = Context::new(&window).await;
        let camera = Camera::new(&window);

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

        let mut world = World::new()
            .with_resource(State::Running)
            .with_resource(window)
            .with_resource(ctx)
            .with_resource(camera)
            .with_resource(assets)
            .with_resource(Mouse::default())
            .with_resource(Keyboard::default())
            .with_ticker(window::clear_mouse_delta)
            .with_ticker(window::poll_events)
            .with_handler(camera::handle_resize)
            .with_handler(graphics::resize_surface)
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
            .with_handler(|world, event| match event {
                Event::Stop => {
                    *world.get_mut::<State>().unwrap() = State::Stopped;
                }
                _ => (),
            })
            .register::<CopperOre>()
            .register::<Tree>();

        world.spawn(CopperOre {
            render: RenderObject {
                mesh: copper_ore,
                material,
            },
        });
        /*world.spawn(Tree {
            render: RenderObject {
                mesh: tree,
                material,
            },
        });*/

        loop {
            if let State::Stopped = *world.get::<State>().unwrap() {
                break;
            }
            world.tick();
        }
    */
}
