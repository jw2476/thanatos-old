use std::collections::VecDeque;

use crate::{window::Window, World};
use hephaestus::{
    command,
    pipeline::{
        self, Framebuffer, ImageLayout, PipelineBindPoint, RenderPass, ShaderModule, Subpass,
    },
    task::{Fence, Semaphore, SubmitInfo, Task},
    ClearColorValue, ClearValue, Context, PipelineStageFlags, VkResult,
};

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
            .fragment(&fragment)
            .render_pass(&render_pass)
            .subpass(0)
            .viewport(
                ctx.swapchain.extent.width as f32,
                ctx.swapchain.extent.height as f32,
            )
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

        Ok(Self {
            ctx,
            render_pass,
            pipeline,
            framebuffers,
            semaphores,
            frame_index: 0,
            tasks: VecDeque::new(),
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
        self.framebuffers
            .into_iter()
            .for_each(|framebuffer| framebuffer.destroy(&self.ctx.device));
        self.pipeline.destroy(&self.ctx.device);
        self.render_pass.destroy(&self.ctx.device);
        self.ctx.destroy();
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
    let render_finished = renderer.semaphores[renderer.frame_index % Renderer::FRAMES_IN_FLIGHT].clone();
    let in_flight = task.fence(&renderer.ctx.device).unwrap();
    let image_index = task
        .acquire_next_image(
            &renderer.ctx.device,
            &renderer.ctx.swapchain,
            image_available.clone(),
        )
        .unwrap();

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

    renderer.tasks.push_back( Frame { task, cmd, fence: in_flight });

    renderer.frame_index += 1;
}
