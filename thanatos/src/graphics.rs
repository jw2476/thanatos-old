use crate::{window::Window, World};
use hephaestus::{
    pipeline::{
        self, Framebuffer, ImageLayout, PipelineBindPoint, RenderPass, ShaderModule, Subpass,
    },
    task::Task,
    ClearColorValue, ClearValue, Context, PipelineStageFlags, VkResult,
};

pub struct Renderer {
    ctx: Context,
    render_pass: RenderPass,
    pipeline: pipeline::Graphics,
    framebuffers: Vec<Framebuffer>,
}

impl Renderer {
    pub fn new(window: &Window) -> VkResult<Self> {
        let ctx = Context::new("thanatos", &window.window)?;

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

        Ok(Self {
            ctx,
            render_pass,
            pipeline,
            framebuffers,
        })
    }

    pub fn destroy(self) {
        self.framebuffers
            .into_iter()
            .for_each(|framebuffer| framebuffer.destroy(&self.ctx.device));
        self.pipeline.destroy(&self.ctx.device);
        self.render_pass.destroy(&self.ctx.device);
        self.ctx.destroy();
    }
}

pub fn draw(world: &mut World) {
    let renderer = world.get::<Renderer>().unwrap();
    let mut task = Task::new();
    let (image_available, image_index) = task
        .acquire_next_image(&renderer.ctx.device, &renderer.ctx.swapchain)
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

    let (render_finished, in_flight) = task
        .submit(
            &renderer.ctx.device,
            &renderer.ctx.device.queues.graphics,
            &cmd,
            &[(image_available, PipelineStageFlags::TOP_OF_PIPE)],
        )
        .unwrap();

    task.present(
        &renderer.ctx.device,
        &renderer.ctx.swapchain,
        image_index,
        &[render_finished],
    )
    .unwrap();

    in_flight.wait(&renderer.ctx.device).unwrap();
    cmd.destroy(&renderer.ctx.device, &renderer.ctx.command_pool);
    task.destroy(&renderer.ctx.device);
}
