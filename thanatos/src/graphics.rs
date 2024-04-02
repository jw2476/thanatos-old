use std::{collections::VecDeque, mem::size_of};

use crate::{window::Window, World};
use bytemuck::offset_of;
use glam::{Vec2, Vec3};
use hephaestus::{
    buffer::Buffer,
    command,
    pipeline::{
        self, Framebuffer, ImageLayout, PipelineBindPoint, RenderPass, ShaderModule, Subpass,
        Viewport,
    },
    task::{Fence, Semaphore, SubmitInfo, Task},
    vertex::{self, AttributeType},
    BufferUsageFlags, ClearColorValue, ClearValue, Context, Extent2D, MemoryPropertyFlags,
    PipelineStageFlags, VkResult,
};
use log::info;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pos: Vec2,
    colour: Vec3,
}

impl Vertex {
    pub fn info() -> vertex::Info {
        vertex::Info::new(size_of::<Self>())
            .attribute(AttributeType::Vec2, 0)
            .attribute(AttributeType::Vec3, offset_of!(Vertex, colour))
    }
}

pub struct Frame {
    task: Task,
    cmd: command::Buffer,
    fence: Fence,
}

pub struct Renderer {
    ctx: Context,
    render_pass: RenderPass,
    pipeline: pipeline::Graphics,
    framebuffers: Vec<Framebuffer>,
    semaphores: Vec<Semaphore>,
    frame_index: usize,
    tasks: VecDeque<Frame>,
    vertex_buffer: Buffer,
}

impl Renderer {
    pub const FRAMES_IN_FLIGHT: usize = 3;

    pub fn new(window: &Window) -> VkResult<Self> {
        let size = window.window.inner_size();
        let ctx = Context::new("thanatos", &window.window, (size.width, size.height))?;

        let vertex = ShaderModule::new(
            &ctx.device,
            &std::fs::read("assets/shaders/shader.vert.spv").unwrap(),
        )?;

        let fragment = ShaderModule::new(
            &ctx.device,
            &std::fs::read("assets/shaders/shader.frag.spv").unwrap(),
        )?;

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
            builder.build(&ctx.device)?
        };

        let pipeline = pipeline::Graphics::builder()
            .vertex(&vertex)
            .vertex_info(Vertex::info())
            .fragment(&fragment)
            .render_pass(&render_pass)
            .subpass(0)
            .viewport(Viewport::Dynamic)
            .build(&ctx.device)?;

        vertex.destroy(&ctx.device);
        fragment.destroy(&ctx.device);

        let framebuffers = ctx
            .swapchain
            .views
            .iter()
            .map(|view| render_pass.get_framebuffer(&ctx.device, &[view]))
            .collect::<VkResult<Vec<Framebuffer>>>()?;

        let semaphores = (0..Self::FRAMES_IN_FLIGHT)
            .map(|_| Semaphore::new(&ctx.device))
            .collect::<VkResult<Vec<Semaphore>>>()?;

        let vertices = [
            Vertex {
                pos: Vec2::new(0.0, -0.5),
                colour: Vec3::X,
            },
            Vertex {
                pos: Vec2::new(0.5, 0.5),
                colour: Vec3::Y,
            },
            Vertex {
                pos: Vec2::new(-0.5, 0.5),
                colour: Vec3::Z,
            },
        ];
        let vertices_data = bytemuck::cast_slice::<Vertex, u8>(&vertices);

        let vertex_buffer = Buffer::new(
            &ctx.instance,
            &ctx.device,
            vertices_data.len(),
            BufferUsageFlags::VERTEX_BUFFER,
            MemoryPropertyFlags::HOST_COHERENT | MemoryPropertyFlags::HOST_VISIBLE,
        )?;
        vertex_buffer.write(&ctx.device, vertices_data)?;

        Ok(Self {
            ctx,
            render_pass,
            pipeline,
            framebuffers,
            semaphores,
            frame_index: 0,
            tasks: VecDeque::new(),
            vertex_buffer
        })
    }

    pub fn destroy(self) {
        unsafe { self.ctx.device.device_wait_idle().unwrap() };
        self.tasks.into_iter().for_each(|frame| {
            frame.cmd.destroy(&self.ctx.device, &self.ctx.command_pool);
            frame.task.destroy(&self.ctx.device);
        });
        self.semaphores
            .into_iter()
            .for_each(|semaphore| semaphore.destroy(&self.ctx.device));
        self.vertex_buffer.destroy(&self.ctx.device);
        self.framebuffers
            .into_iter()
            .for_each(|framebuffer| framebuffer.destroy(&self.ctx.device));
        self.pipeline.destroy(&self.ctx.device);
        self.render_pass.destroy(&self.ctx.device);
        self.ctx.destroy();
    }

    pub fn recreate_swapchain(&mut self, size: (u32, u32)) -> VkResult<()> {
        self.ctx.surface.extent = Extent2D {
            width: size.0,
            height: size.1,
        };
        self.ctx.recreate_swapchain().unwrap();
        self.framebuffers
            .drain(..)
            .for_each(|framebuffer| framebuffer.destroy(&self.ctx.device));
        self.framebuffers = self
            .ctx
            .swapchain
            .views
            .iter()
            .map(|view| self.render_pass.get_framebuffer(&self.ctx.device, &[view]))
            .collect::<VkResult<Vec<Framebuffer>>>()?;
        Ok(())
    }
}

pub fn draw(world: &mut World) {
    let mut renderer = world.get_mut::<Renderer>().unwrap();
    if renderer.tasks.len() > Renderer::FRAMES_IN_FLIGHT {
        let frame = renderer.tasks.pop_front().unwrap();
        frame.fence.wait(&renderer.ctx.device).unwrap();
        frame
            .cmd
            .destroy(&renderer.ctx.device, &renderer.ctx.command_pool);
        frame.task.destroy(&renderer.ctx.device);
    }

    let mut task = Task::new();
    let image_available = task.semaphore(&renderer.ctx.device).unwrap();
    let render_finished =
        renderer.semaphores[renderer.frame_index % Renderer::FRAMES_IN_FLIGHT].clone();
    let in_flight = task.fence(&renderer.ctx.device).unwrap();
    let (image_index, suboptimal) = task
        .acquire_next_image(
            &renderer.ctx.device,
            &renderer.ctx.swapchain,
            image_available.clone(),
        )
        .unwrap();

    let window = world.get::<Window>().unwrap();
    let size = window.window.inner_size();

    if suboptimal {
        info!("Recreating swapchain...");
        unsafe { renderer.ctx.device.device_wait_idle().unwrap() };
        renderer
            .recreate_swapchain((size.width, size.height))
            .unwrap();
        task.destroy(&renderer.ctx.device);
        return;
    }

    let clear_value = ClearValue {
        color: ClearColorValue {
            float32: [0.0, 0.0, 0.0, 1.0],
        },
    };

    let cmd = renderer
        .ctx
        .command_pool
        .alloc(&renderer.ctx.device)
        .unwrap()
        .begin(&renderer.ctx.device)
        .unwrap()
        .begin_render_pass(
            &renderer.render_pass,
            renderer.framebuffers.get(image_index as usize).unwrap(),
            &[clear_value],
        )
        .bind_graphics_pipeline(&renderer.pipeline)
        .set_viewport(size.width, size.height)
        .set_scissor(size.width, size.height)
        .bind_vertex_buffer(&renderer.vertex_buffer, 0)
        .draw(3, 1, 0, 0)
        .end_render_pass()
        .end()
        .unwrap();

    task.submit(SubmitInfo {
        device: &renderer.ctx.device,
        queue: &renderer.ctx.device.queues.graphics,
        cmd: &cmd,
        wait: &[(image_available, PipelineStageFlags::TOP_OF_PIPE)],
        signal: &[render_finished.clone()],
        fence: in_flight.clone(),
    })
    .unwrap();

    task.present(
        &renderer.ctx.device,
        &renderer.ctx.swapchain,
        image_index,
        &[render_finished],
    )
    .unwrap();

    renderer.tasks.push_back(Frame {
        task,
        cmd,
        fence: in_flight,
    });

    renderer.frame_index += 1;
}
