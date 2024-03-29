use std::{
    collections::HashSet,
    ffi::{c_char, CStr, CString},
    ops::Deref,
};

use ash::{
    prelude::VkResult,
    vk::{
        self, ApplicationInfo, ColorSpaceKHR, ComponentMapping, ComponentMappingBuilder,
        ComponentSwizzle, CompositeAlphaFlagsKHR, DeviceCreateInfo, DeviceQueueCreateInfo,
        Extent2D, Format, Image, ImageAspectFlags, ImageSubresourceRange, ImageUsageFlags,
        ImageView, ImageViewCreateInfo, ImageViewType, InstanceCreateInfo, PhysicalDeviceFeatures,
        PhysicalDeviceProperties, PresentModeKHR, QueueFamilyProperties, QueueFlags, SharingMode,
        SurfaceCapabilitiesKHR, SurfaceFormatKHR, SwapchainCreateInfoKHR, SwapchainKHR,
    },
    Entry,
};
use log::{error, warn};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

pub struct InstanceExtensions {
    pub surface: ash::extensions::khr::Surface,
}

impl InstanceExtensions {
    pub fn new(entry: &Entry, instance: &ash::Instance) -> Self {
        let surface = ash::extensions::khr::Surface::new(entry, &instance);

        Self { surface }
    }
}

pub struct Instance {
    pub inner: ash::Instance,
    pub extensions: InstanceExtensions,
}

impl Deref for Instance {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct PhysicalDevice {
    pub handle: vk::PhysicalDevice,
    pub properties: PhysicalDeviceProperties,
    pub features: PhysicalDeviceFeatures,
    pub queue_families: Vec<QueueFamilyProperties>,
}

pub struct Surface {
    pub handle: vk::SurfaceKHR,
    pub capabilities: SurfaceCapabilitiesKHR,
    pub formats: Vec<SurfaceFormatKHR>,
    pub present_modes: Vec<PresentModeKHR>,
}

impl Instance {
    #[cfg(target_os = "linux")]
    const EXTENSIONS: &'static [&'static CStr] = &[
        ash::extensions::khr::Surface::name(),
        ash::extensions::khr::XcbSurface::name(),
    ];

    const LAYERS: &'static [&'static CStr] = &[c"VK_LAYER_KHRONOS_validation"];

    pub unsafe fn new<T: HasRawDisplayHandle>(
        entry: &Entry,
        name: &CStr,
        window: T,
    ) -> VkResult<Self> {
        let app_info = ApplicationInfo::builder()
            .engine_name(name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .application_name(name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_3);

        let available = entry.enumerate_instance_layer_properties()?;
        let layers = Self::LAYERS
            .iter()
            .filter(|wanted| {
                let found = available
                    .iter()
                    .find(|layer| CStr::from_ptr(layer.layer_name.as_ptr()) == **wanted)
                    .is_some();
                if !found {
                    warn!("Missing validation layer: {}", wanted.to_str().unwrap())
                }
                found
            })
            .map(|name| name.as_ptr() as *const c_char)
            .collect::<Vec<_>>();

        let available = entry.enumerate_instance_extension_properties(None)?;
        let presentation_extensions =
            ash_window::enumerate_required_extensions(window.raw_display_handle())?;
        let extensions = Self::EXTENSIONS
            .iter()
            .filter(|wanted| {
                let found = available
                    .iter()
                    .find(|extension| CStr::from_ptr(extension.extension_name.as_ptr()) == **wanted)
                    .is_some();
                if !found {
                    error!("Missing extension: {}", wanted.to_str().unwrap())
                }
                found
            })
            .map(|name| name.as_ptr() as *const c_char)
            .chain(presentation_extensions.iter().map(|x| *x))
            .collect::<Vec<_>>();

        let create_info = InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layers);

        let inner = entry.create_instance(&create_info, None)?;
        let extensions = InstanceExtensions::new(entry, &inner);
        Ok(Self { inner, extensions })
    }

    pub unsafe fn get_physical_device(&self) -> VkResult<PhysicalDevice> {
        let devices = self.enumerate_physical_devices()?;
        let handle = *devices.first().expect("No device found");
        let properties = self.get_physical_device_properties(handle);
        let features = self.get_physical_device_features(handle);
        let queue_families = self.get_physical_device_queue_family_properties(handle);
        Ok(PhysicalDevice {
            handle,
            properties,
            features,
            queue_families,
        })
    }

    pub unsafe fn get_surface<T: HasRawDisplayHandle + HasRawWindowHandle>(
        &self,
        entry: &Entry,
        instance: &Instance,
        physical: &PhysicalDevice,
        window: T,
    ) -> VkResult<Surface> {
        let handle = ash_window::create_surface(
            entry,
            self,
            window.raw_display_handle(),
            window.raw_window_handle(),
            None,
        )?;

        let capabilities = instance
            .extensions
            .surface
            .get_physical_device_surface_capabilities(physical.handle, handle)?;
        let formats = instance
            .extensions
            .surface
            .get_physical_device_surface_formats(physical.handle, handle)?;
        let present_modes = instance
            .extensions
            .surface
            .get_physical_device_surface_present_modes(physical.handle, handle)?;

        Ok(Surface {
            handle,
            capabilities,
            formats,
            present_modes,
        })
    }
}

pub struct Queue {
    pub handle: vk::Queue,
    pub index: u32,
}

impl Queue {
    pub unsafe fn get(device: &ash::Device, index: u32) -> Self {
        let handle = device.get_device_queue(index, 0);
        Self { handle, index }
    }
}

pub struct Queues {
    pub graphics: Queue,
    pub present: Queue,
}

pub struct DeviceExtensions {
    pub swapchain: ash::extensions::khr::Swapchain,
}

pub struct Device {
    pub inner: ash::Device,
    pub extensions: DeviceExtensions,
    pub physical: PhysicalDevice,
    pub queues: Queues,
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct Swapchain {
    handle: SwapchainKHR,
    images: Vec<Image>,
    views: Vec<ImageView>,
    format: Format,
    extent: Extent2D,
}

impl Device {
    const EXTENSIONS: &'static [&'static CStr] = &[ash::extensions::khr::Swapchain::name()];

    pub unsafe fn new(
        instance: &Instance,
        physical: PhysicalDevice,
        surface: &Surface,
    ) -> VkResult<Self> {
        let priorities = &[1.0];

        let graphics_index = physical
            .queue_families
            .iter()
            .position(|family| family.queue_flags.contains(QueueFlags::GRAPHICS))
            .expect("No graphics capable queue families") as u32;
        let present_index = physical
            .queue_families
            .iter()
            .enumerate()
            .position(|(i, _)| {
                instance
                    .extensions
                    .surface
                    .get_physical_device_surface_support(physical.handle, i as u32, surface.handle)
                    .unwrap()
            })
            .expect("No presentation capable queue families") as u32;

        let indices = HashSet::from([graphics_index, present_index]);
        let queue_create_infos = indices
            .into_iter()
            .map(|index| {
                DeviceQueueCreateInfo::builder()
                    .queue_family_index(index)
                    .queue_priorities(priorities)
                    .build()
            })
            .collect::<Vec<_>>();

        let available = instance.enumerate_device_extension_properties(physical.handle)?;
        let extensions = Self::EXTENSIONS
            .iter()
            .filter(|wanted| {
                let found = available
                    .iter()
                    .find(|extension| CStr::from_ptr(extension.extension_name.as_ptr()) == **wanted)
                    .is_some();
                if !found {
                    error!("Missing extension: {}", wanted.to_str().unwrap())
                }
                found
            })
            .map(|name| name.as_ptr() as *const c_char)
            .collect::<Vec<_>>();

        let create_info = DeviceCreateInfo::builder()
            .enabled_extension_names(&extensions)
            .queue_create_infos(&queue_create_infos);

        let inner = instance.create_device(physical.handle, &create_info, None)?;

        let queues = Queues {
            graphics: Queue::get(&inner, graphics_index),
            present: Queue::get(&inner, present_index),
        };

        let swapchain = ash::extensions::khr::Swapchain::new(&instance, &inner);
        let extensions = DeviceExtensions { swapchain };

        Ok(Self {
            inner,
            extensions,
            physical,
            queues,
        })
    }

    pub unsafe fn get_swapchain(&self, surface: &Surface) -> VkResult<Swapchain> {
        let format = surface
            .formats
            .iter()
            .find(|format| {
                format.format == Format::B8G8R8_SRGB
                    && format.color_space == ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or_else(|| surface.formats.first().unwrap());

        let present_mode = surface
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == PresentModeKHR::MAILBOX)
            .unwrap_or(PresentModeKHR::FIFO);

        let extent = if surface.capabilities.current_extent.width == u32::MAX {
            todo!()
        } else {
            surface.capabilities.current_extent
        };

        let image_count = if surface.capabilities.max_image_count == 0 {
            surface.capabilities.min_image_count + 1
        } else {
            (surface.capabilities.min_image_count + 1).min(surface.capabilities.max_image_count)
        };

        let create_info = SwapchainCreateInfoKHR::builder()
            .surface(surface.handle)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(surface.capabilities.current_transform)
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        let indices = [self.queues.graphics.index, self.queues.present.index];
        let create_info = if self.queues.graphics.index == self.queues.present.index {
            create_info.image_sharing_mode(SharingMode::EXCLUSIVE)
        } else {
            create_info
                .image_sharing_mode(SharingMode::CONCURRENT)
                .queue_family_indices(&indices)
        };

        let handle = self
            .extensions
            .swapchain
            .create_swapchain(&create_info, None)?;

        let images = self.extensions.swapchain.get_swapchain_images(handle)?;
        let views = images
            .iter()
            .map(|image| {
                let create_info = ImageViewCreateInfo::builder()
                    .image(*image)
                    .view_type(ImageViewType::TYPE_2D)
                    .format(format.format)
                    .components(ComponentMapping::default())
                    .subresource_range(
                        ImageSubresourceRange::builder()
                            .aspect_mask(ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1)
                            .build(),
                    );
                self.create_image_view(&create_info, None)
            })
            .collect::<VkResult<Vec<_>>>()?;

        Ok(Swapchain {
            handle,
            images,
            views,
            format: format.format,
            extent,
        })
    }
}

pub struct Context {
    entry: Entry,
    pub instance: Instance,
    pub surface: Surface,
    pub device: Device,
    pub swapchain: Swapchain,
}

impl Context {
    pub fn new<T: HasRawWindowHandle + HasRawDisplayHandle>(
        name: &str,
        window: T,
    ) -> VkResult<Self> {
        let entry = Entry::linked();
        let name = CString::new(name).unwrap();
        let instance = unsafe { Instance::new(&entry, &name, &window)? };
        let physical = unsafe { instance.get_physical_device()? };
        let surface = unsafe { instance.get_surface(&entry, &instance, &physical, window)? };
        let device = unsafe { Device::new(&instance, physical, &surface)? };
        let swapchain = unsafe { device.get_swapchain(&surface)? };

        Ok(Self {
            entry,
            instance,
            surface,
            device,
            swapchain,
        })
    }

    unsafe fn destroy(&self) {
        self.swapchain
            .views
            .iter()
            .for_each(|view| self.device.destroy_image_view(*view, None));
        self.device
            .extensions
            .swapchain
            .destroy_swapchain(self.swapchain.handle, None);
        self.device.destroy_device(None);

        self.instance
            .extensions
            .surface
            .destroy_surface(self.surface.handle, None);
        self.instance.destroy_instance(None);
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { self.destroy() }
    }
}
