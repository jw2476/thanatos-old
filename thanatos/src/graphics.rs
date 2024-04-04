use std::{collections::VecDeque, mem::size_of};

use crate::{camera::Camera, window::Window, World};
use bytemuck::offset_of;
use glam::{Vec2, Vec3};
use hephaestus::{
    buffer::Static, command, descriptor, pipeline::{
        self, Framebuffer, ImageLayout, PipelineBindPoint, RenderPass, ShaderModule, Subpass,
        Viewport,
    }, task::{Fence, Semaphore, SubmitInfo, Task}, vertex::{self, AttributeType}, BufferUsageFlags, ClearColorValue, ClearValue, Context, DescriptorType, Extent2D, PipelineStageFlags, VkResult
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
    camera_buffer: Static,
    camera_set: descriptor::Set
}

impl Frame {
    pub fn destroy(self, ctx: &Context) {
        self.fence.wait(&ctx.device).unwrap();
        self
            .cmd
            .destroy(&ctx.device, &ctx.command_pool);
        self.camera_set.destroy(&ctx);
        self.camera_buffer.destroy(&ctx.device);
        self.task.destroy(&ctx.device);
    }
}

pub struct Renderer {
    ctx: Context,
    render_pass: RenderPass,
    pipeline: pipeline::Graphics,
    framebuffers: Vec<Framebuffer>,
    semaphores: Vec<Semaphore>,
    frame_index: usize,
    tasks: VecDeque<Frame>,
    vertex_buffer: Static,
    index_buffer: Static,
    camera_layout: descriptor::Layout
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

        let camera_layout = descriptor::Layout::new(&ctx, &[DescriptorType::UNIFORM_BUFFER], 1000)?;

        let pipeline = pipeline::Graphics::builder()
            .vertex(&vertex)
            .vertex_info(Vertex::info())
            .fragment(&fragment)
            .render_pass(&render_pass)
            .subpass(0)
            .viewport(Viewport::Dynamic)
            .layouts(vec![&camera_layout])
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
                pos: Vec2::new(-0.5, -0.5),
                colour: Vec3::X,
            },
            Vertex {
                pos: Vec2::new(0.5, -0.5),
                colour: Vec3::Y,
            },
            Vertex {
                pos: Vec2::new(0.5, 0.5),
                colour: Vec3::Z,
            },
            Vertex {
                pos: Vec2::new(-0.5, 0.5),
                colour: Vec3::ONE,
            },
        ];

        let indices = [0, 1, 2, 2, 3, 0];

        let vertices_data = bytemuck::cast_slice::<Vertex, u8>(&vertices);
        let indices_data = bytemuck::cast_slice::<u32, u8>(&indices);
        let vertex_buffer = Static::new(&ctx, vertices_data, BufferUsageFlags::VERTEX_BUFFER)?;
        let index_buffer = Static::new(&ctx, indices_data, BufferUsageFlags::INDEX_BUFFER)?;

        Ok(Self {
            ctx,
            render_pass,
            pipeline,
            framebuffers,
            semaphores,
            frame_index: 0,
            tasks: VecDeque::new(),
            vertex_buffer,
            index_buffer,
            camera_layout
        })
    }

    pub fn destroy(self) {
        unsafe { self.ctx.device.device_wait_idle().unwrap() };
        self.tasks.into_iter().for_each(|frame| frame.destroy(&self.ctx));
        self.semaphores
            .into_iter()
            .for_each(|semaphore| semaphore.destroy(&self.ctx.device));
        self.vertex_buffer.destroy(&self.ctx.device);
        self.index_buffer.destroy(&self.ctx.device);
        self.framebuffers
            .into_iter()
            .for_each(|framebuffer| framebuffer.destroy(&self.ctx.device));
        self.pipeline.destroy(&self.ctx.device);
        self.camera_layout.destroy(&self.ctx);
        self.render_pass.destroy(&self.ctx.device);
        self.ctx.destroy();
    }

    pub fn recreate_swapchain(&mut self, size: (u32, u32)) -> VkResult<()> {
        unsafe { self.ctx.device.device_wait_idle()? }
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
        frame.destroy(&renderer.ctx);
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
        info!("Recreating swapchain");
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

    let camera = world.get::<Camera>().unwrap();
    let camera_buffer = Static::new(
        &renderer.ctx,
        bytemuck::cast_slice::<f32, u8>(&camera.get_matrix().to_cols_array()),
        BufferUsageFlags::UNIFORM_BUFFER,
    )
    .unwrap();
    let camera_set = renderer.camera_layout.alloc(&renderer.ctx).unwrap();
    camera_set.write_buffer(&renderer.ctx, 0, &camera_buffer);

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
        .bind_index_buffer(&renderer.index_buffer)
        .bind_descriptor_set(&camera_set, 0)
        .draw_indexed(6, 1, 0, 0, 0)
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

    let suboptimal = task.present(
        &renderer.ctx.device,
        &renderer.ctx.swapchain,
        image_index,
        &[render_finished],
    )
    .unwrap();


    if suboptimal {
        info!("Recreating swapchain");
        renderer
            .recreate_swapchain((size.width, size.height))
            .unwrap();
    }

    renderer.tasks.push_back(Frame {
        task,
        cmd,
        fence: in_flight,
        camera_buffer,
        camera_set
    });

    renderer.frame_index += 1;
}
