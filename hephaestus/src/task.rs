use ash::{
    prelude::VkResult,
    vk::{self, FenceCreateInfo, PipelineStageFlags, PresentInfoKHR, SemaphoreCreateInfo},
};

use crate::{command, Device, Queue, Swapchain};

#[derive(Clone)]
pub struct Fence {
    pub handle: vk::Fence,
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

#[derive(Clone)]
pub struct Semaphore {
    pub handle: vk::Semaphore,
}

impl Semaphore {
    pub fn new(device: &Device) -> VkResult<Self> {
        let create_info = SemaphoreCreateInfo::default();
        let handle = unsafe { device.create_semaphore(&create_info, None)? };
        Ok(Self { handle })
    }

    pub fn destroy(self, device: &Device) {
        unsafe { device.destroy_semaphore(self.handle, None) }
    }
}

#[derive(Default)]
pub struct Task {
    semaphores: Vec<Semaphore>,
    fences: Vec<Fence>,
}

pub struct SubmitInfo<'a> {
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub cmd: &'a command::Buffer,
    pub wait: &'a [(Semaphore, PipelineStageFlags)],
    pub signal: &'a [Semaphore],
    pub fence: Fence,
}

impl Task {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn semaphore(&mut self, device: &Device) -> VkResult<Semaphore> {
        let semaphore = Semaphore::new(device)?;
        self.semaphores.push(semaphore.clone());
        Ok(semaphore)
    }

    pub fn fence(&mut self, device: &Device) -> VkResult<Fence> {
        let fence = Fence::new(device)?;
        self.fences.push(fence.clone());
        Ok(fence)
    }

    pub fn acquire_next_image(
        &mut self,
        device: &Device,
        swapchain: &Swapchain,
        signal: Semaphore,
    ) -> VkResult<(u32, bool)> {
        let result = unsafe {
            device.extensions.swapchain.acquire_next_image(
                swapchain.handle,
                u64::MAX,
                signal.handle,
                vk::Fence::null(),
            )
        };
        match result {
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok((0, true)),
            x => x,
        }
    }

    pub fn submit(&mut self, info: SubmitInfo) -> VkResult<()> {
        let stages = info
            .wait
            .iter()
            .map(|(_, stage)| *stage)
            .collect::<Vec<_>>();
        let wait_semaphores = info
            .wait
            .iter()
            .map(|(semaphore, _)| semaphore.handle)
            .collect::<Vec<_>>();
        let buffers = [info.cmd.handle];
        let signal_semaphores = info
            .signal
            .iter()
            .map(|semamphore| semamphore.handle)
            .collect::<Vec<_>>();

        let submit_info = vk::SubmitInfo::builder()
            .wait_dst_stage_mask(&stages)
            .wait_semaphores(&wait_semaphores)
            .command_buffers(&buffers)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            info.device
                .queue_submit(info.queue.handle, &[*submit_info], info.fence.handle)?
        };
        Ok(())
    }

    pub fn present(
        &mut self,
        device: &Device,
        swapchain: &Swapchain,
        image_index: u32,
        wait: &[Semaphore],
    ) -> VkResult<bool> {
        let wait_semaphores = wait.iter().map(|wait| wait.handle).collect::<Vec<_>>();
        let swapchains = [swapchain.handle];
        let image_indices = [image_index];
        let present_info = PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        let result = unsafe {
            device
                .extensions
                .swapchain
                .queue_present(device.queues.present.handle, &present_info)
        };
        match result {
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(true),
            Err(vk::Result::SUBOPTIMAL_KHR) => Ok(true),
            x => x,
        }
    }

    pub fn destroy(self, device: &Device) {
        self.semaphores
            .into_iter()
            .for_each(|semaphore| semaphore.destroy(device));
        self.fences
            .into_iter()
            .for_each(|fence| fence.destroy(device));
    }
}
