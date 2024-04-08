use ash::{
    prelude::VkResult,
    vk::{
        self, DescriptorBufferInfo, DescriptorPoolCreateFlags, DescriptorPoolCreateInfo,
        DescriptorPoolSize, DescriptorSetAllocateInfo, DescriptorSetLayoutBinding,
        DescriptorSetLayoutCreateInfo, DescriptorType, ShaderStageFlags, WriteDescriptorSet,
    },
};

use crate::{buffer, Context};

#[derive(Clone)]
pub struct Layout {
    pub layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
    pub bindings: Vec<DescriptorType>,
}

pub struct Set {
    layout: Layout,
    pub handle: vk::DescriptorSet,
}

impl Layout {
    pub fn new(ctx: &Context, bindings: &[DescriptorType], capacity: usize) -> VkResult<Self> {
        let binding_infos = bindings
            .iter()
            .enumerate()
            .map(|(i, ty)| {
                DescriptorSetLayoutBinding::builder()
                    .binding(i as u32)
                    .descriptor_type(*ty)
                    .descriptor_count(1)
                    .stage_flags(ShaderStageFlags::ALL)
                    .build()
            })
            .collect::<Vec<_>>();
        let create_info = DescriptorSetLayoutCreateInfo::builder().bindings(&binding_infos);
        let layout = unsafe {
            ctx.device
                .create_descriptor_set_layout(&create_info, None)?
        };

        let pool_sizes = bindings
            .iter()
            .map(|ty| {
                DescriptorPoolSize::builder()
                    .ty(*ty)
                    .descriptor_count(capacity as u32)
                    .build()
            })
            .collect::<Vec<_>>();

        let create_info = DescriptorPoolCreateInfo::builder()
            .flags(DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .pool_sizes(&pool_sizes)
            .max_sets(capacity as u32);
        let pool = unsafe { ctx.device.create_descriptor_pool(&create_info, None)? };

        Ok(Self {
            layout,
            pool,
            bindings: bindings.to_vec(),
        })
    }

    pub fn alloc(&self, ctx: &Context) -> VkResult<Set> {
        let set_layouts = [self.layout];
        let alloc_info = DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.pool)
            .set_layouts(&set_layouts);
        let handle = unsafe { ctx.device.allocate_descriptor_sets(&alloc_info)?[0] };
        Ok(Set {
            handle,
            layout: self.clone(),
        })
    }

    pub fn destroy(self, ctx: &Context) {
        unsafe { ctx.device.destroy_descriptor_pool(self.pool, None) }
        unsafe { ctx.device.destroy_descriptor_set_layout(self.layout, None) }
    }
}

impl Set {
    pub fn write_buffer<T: buffer::Buffer>(&self, ctx: &Context, binding: usize, buffer: &T) {
        let buffer_info = DescriptorBufferInfo {
            buffer: buffer.buffer(),
            offset: 0,
            range: buffer.size() as u64,
        };
        let buffer_infos = [buffer_info];

        let write_info = WriteDescriptorSet::builder()
            .dst_set(self.handle)
            .dst_binding(binding as u32)
            .dst_array_element(0)
            .descriptor_type(self.layout.bindings[binding])
            .buffer_info(&buffer_infos);
        unsafe { ctx.device.update_descriptor_sets(&[*write_info], &[]) }
    }

    pub fn destroy(self, ctx: &Context) {
        unsafe {
            ctx.device
                .free_descriptor_sets(self.layout.pool, &[self.handle])
                .unwrap()
        }
    }
}
