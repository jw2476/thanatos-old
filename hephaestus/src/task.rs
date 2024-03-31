use ash::{prelude::VkResult, vk::{self, FenceCreateInfo, PipelineStageFlags, PresentInfoKHR, Semaphore, SemaphoreCreateInfo, SubmitInfo}};

use crate::{command, Device, Queue, Swapchain};

#[derive(Clone)]
pub struct Fence {
    pub handle: vk::Fence
}

impl Fence {
    pub fn new(device: &Device) -> VkResult<Self> {
        let create_info = FenceCreateInfo::default();
        let handle = unsafe { device.create_fence(&create_info, None)? };
        Ok(Self { handle })
    }

    pub fn wait(&self, device: &Device) -> VkResult<()> {
        let fences = [self.handle];
        unsafe { device.wait_for_fences(&fences, true, u64::MAX) }
    }

    pub fn reset(&self, device: &Device) -> VkResult<()> {
        let fences = [self.handle];
        unsafe { device.reset_fences(&fences) }
    }

    pub fn destroy(self, device: &Device) {
        unsafe { device.destroy_fence(self.handle, None) }
    }
}

#[derive(Default)]
pub struct Task {
    semaphores: Vec<Semaphore>,
    fences: Vec<Fence>,
}

impl Task {
    pub fn new() -> Self {
        Self::default()
    }

    fn semaphore(&mut self, device: &Device) -> VkResult<Semaphore> {
        let create_info = SemaphoreCreateInfo::default();
        let semaphore = unsafe { device.create_semaphore(&create_info, None)? };
        self.semaphores.push(semaphore);
        Ok(semaphore)
    }

    fn fence(&mut self, device: &Device) -> VkResult<Fence> {
        let fence = Fence::new(device)?;
        self.fences.push(fence.clone());
        Ok(fence)
    }

    pub fn acquire_next_image(&mut self, device: &Device, swapchain: &Swapchain) -> VkResult<(Semaphore, u32)> {
        let semaphore = self.semaphore(device)?;
        let (image_index, _) = unsafe { device.extensions.swapchain.acquire_next_image(swapchain.handle, u64::MAX, semaphore, vk::Fence::null())? };
        Ok((semaphore, image_index))
    }

    pub fn submit(&mut self, device: &Device, queue: &Queue, cmd: &command::Buffer, wait: &[(Semaphore, PipelineStageFlags)]) -> VkResult<(Semaphore, Fence)> {
        let semaphores = [self.semaphore(device)?];
        let fence = self.fence(device)?;
        let stages = wait.iter().map(|(_, stage)| *stage).collect::<Vec<_>>();
        let wait_semaphores = wait.iter().map(|(semaphore, _)| *semaphore).collect::<Vec<_>>();
        let buffers = [cmd.handle];

        let submit_info = SubmitInfo::builder()
            .wait_dst_stage_mask(&stages)
            .wait_semaphores(&wait_semaphores)
            .command_buffers(&buffers)
            .signal_semaphores(&semaphores);

        unsafe { device.queue_submit(queue.handle, &[*submit_info], fence.handle)? };
        Ok((semaphores[0], fence))
    }

    pub fn present(&mut self, device: &Device, swapchain: &Swapchain, image_index: u32, wait: &[Semaphore]) -> VkResult<()> {
        let swapchains = [swapchain.handle];
        let image_indices = [image_index];
        let present_info = PresentInfoKHR::builder()
            .wait_semaphores(wait)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        unsafe { device.extensions.swapchain.queue_present(device.queues.present.handle, &present_info)? };
        Ok(())
    }

    pub fn destroy(self, device: &Device) {
        self.semaphores
            .into_iter()
            .for_each(|semaphore| unsafe { device.destroy_semaphore(semaphore, None) });
        self.fences
            .into_iter()
            .for_each(|fence| fence.destroy(device));
    }
}
