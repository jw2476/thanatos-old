use core::slice;
use std::ffi::c_void;

use ash::{
    prelude::VkResult,
    vk::{self, BufferCreateInfo, BufferUsageFlags, MemoryAllocateInfo, MemoryMapFlags, MemoryPropertyFlags, SharingMode},
};

use crate::{Device, Instance};

pub struct Buffer {
    pub handle: vk::Buffer,
    pub memory: vk::DeviceMemory
}

impl Buffer {
    pub fn new(
        instance: &Instance,
        device: &Device,
        size: usize,
        usage: BufferUsageFlags,
        wanted: MemoryPropertyFlags,
    ) -> VkResult<Self> {
        let create_info = BufferCreateInfo::builder()
            .size(size as u64)
            .usage(usage)
            .sharing_mode(SharingMode::EXCLUSIVE);
        let handle = unsafe { device.create_buffer(&create_info, None)? };

        let requirements = unsafe { device.get_buffer_memory_requirements(handle) };
        let properties =
            unsafe { instance.get_physical_device_memory_properties(device.physical.handle) };

        let type_index = properties
            .memory_types
            .iter()
            .enumerate()
            .position(|(i, ty)| {
                (requirements.memory_type_bits & (1 << i)) != 0
                    && ty.property_flags.contains(wanted)
            })
            .expect("No memory type found that satisfies requirements");

        let alloc_info = MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(type_index as u32);
        let memory = unsafe { device.allocate_memory(&alloc_info, None)? };
        unsafe { device.bind_buffer_memory(handle, memory, 0)? };

        Ok(Self { handle, memory })
    }

    pub fn write(&self, device: &Device, data: &[u8]) -> VkResult<()> {
        let memory: *mut c_void = unsafe { device.map_memory(self.memory, 0, data.len() as u64, MemoryMapFlags::default())? }; 
        let memory: *mut u8 = memory.cast();
        unsafe { slice::from_raw_parts_mut(memory, data.len()).copy_from_slice(data) };
        unsafe { device.unmap_memory(self.memory) };

        Ok(())
    }

    pub fn destroy(self, device: &Device) {
        unsafe { device.destroy_buffer(self.handle, None) }
        unsafe { device.free_memory(self.memory, None) }
    }
}
