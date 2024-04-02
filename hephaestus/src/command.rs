use ash::{
    prelude::VkResult,
    vk::{self, ClearValue, CommandBufferAllocateInfo, CommandBufferBeginInfo, CommandBufferLevel, CommandPoolCreateInfo, Extent2D, Offset2D, PipelineBindPoint, Rect2D, RenderPassBeginInfo, SubpassContents, Viewport},
};

use crate::{pipeline::{Framebuffer, Graphics, RenderPass}, Device, Queue};

pub struct Buffer {
    pub handle: vk::CommandBuffer,
}

impl Buffer {
    pub fn begin<'a>(self, device: &'a Device) -> VkResult<Recorder<'a>> {
        let begin_info = CommandBufferBeginInfo::default();
        unsafe { device.begin_command_buffer(self.handle, &begin_info)? };
        Ok(Recorder { buffer: self, device })
    }

    pub fn destroy(self, device: &Device, pool: &Pool) {
        let buffers = [self.handle];
        unsafe { device.free_command_buffers(pool.handle, &buffers) }
    }
}

pub struct Recorder<'a> {
    buffer: Buffer,
    device: &'a Device 
}

impl Recorder<'_> {
    pub fn end(self) -> VkResult<Buffer> {
        unsafe { self.device.end_command_buffer(self.buffer.handle)? };
        Ok(self.buffer)
    }

    pub fn begin_render_pass(self, render_pass: &RenderPass, framebuffer: &Framebuffer, clear_values: &[ClearValue]) -> Self {
        let render_pass_begin = RenderPassBeginInfo::builder()
            .render_pass(render_pass.handle)
            .framebuffer(framebuffer.handle)
            .render_area(Rect2D { offset: Offset2D { x: 0, y: 0 }, extent: framebuffer.extent })
            .clear_values(clear_values);
        unsafe { self.device.cmd_begin_render_pass(self.buffer.handle, &render_pass_begin, SubpassContents::INLINE) };
        self
    }

    pub fn end_render_pass(self) -> Self {
        unsafe { self.device.cmd_end_render_pass(self.buffer.handle) };
        self
    }

    pub fn bind_graphics_pipeline(self, pipeline: &Graphics) -> Self {
        unsafe { self.device.cmd_bind_pipeline(self.buffer.handle, PipelineBindPoint::GRAPHICS, pipeline.handle) };
        self
    }

    pub fn draw(self, vertices: u32, instances: u32, first_vertex: u32, first_instance: u32) -> Self {
        unsafe { self.device.cmd_draw(self.buffer.handle, vertices, instances, first_vertex, first_instance) };
        self
    }

    pub fn set_viewport(self, width: u32, height: u32) -> Self {
        let viewport = Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(width as f32)
            .height(height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build();
        let viewports = [viewport];
        unsafe { self.device.cmd_set_viewport(self.buffer.handle, 0, &viewports) }
        self
    } 

    pub fn set_scissor(self, width: u32, height: u32) -> Self {
        let scissor = Rect2D::builder()
            .offset(Offset2D { x: 0, y: 0 })
            .extent(Extent2D { width, height })
            .build();
        let scissors = [scissor];
        unsafe { self.device.cmd_set_scissor(self.buffer.handle, 0, &scissors) }
        self
    }
}

pub struct Pool {
    pub handle: vk::CommandPool,
}

impl Pool {
    pub fn new(device: &Device, queue: &Queue) -> VkResult<Self> {
        let create_info = CommandPoolCreateInfo::builder().queue_family_index(queue.index);
        let handle = unsafe { device.create_command_pool(&create_info, None)? };
        Ok(Self { handle })
    }

    pub fn alloc(&self, device: &Device) -> VkResult<Buffer> {
        let alloc_info = CommandBufferAllocateInfo::builder()
            .command_pool(self.handle)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let handles = unsafe { device.allocate_command_buffers(&alloc_info)? };
        let handle = *handles.first().unwrap();
        Ok(Buffer { handle })
    }

    pub fn destroy(self, device: &Device) {
        unsafe { device.destroy_command_pool(self.handle, None) }
    }
}
