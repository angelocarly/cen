use std::sync::{Arc, Mutex};
use ash::vk;
use ash::vk::{ComponentMapping, ImageAspectFlags};
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationScheme};
use crate::vulkan::{Allocator, Device};
use crate::vulkan::allocator::AllocatorInner;
use crate::vulkan::device::DeviceInner;

pub struct Image {
    pub device_dep: Arc<DeviceInner>,
    pub allocator_dep: Arc<Mutex<AllocatorInner>>,
    pub(crate) image: vk::Image,
    pub(crate) image_view: vk::ImageView,
    pub(crate) sampler: vk::Sampler,
    pub allocation: Allocation,
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.device_dep.device.destroy_sampler(self.sampler, None);
            self.device_dep.device.destroy_image_view(self.image_view, None);
            self.allocator_dep.lock().unwrap().allocator.free(self.allocation);
            self.device_dep.device.destroy_image(self.image, None);
        }
    }
}

impl Image {
    pub fn new(device: &Device, allocator: &mut Allocator, width: u32, height: u32) -> Image {
        let create_info = vk::ImageCreateInfo::default()
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .samples(vk::SampleCountFlags::TYPE_1)
            .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .array_layers(1)
            .mip_levels(1)
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM);

        let image = unsafe {
            device.handle().create_image(&create_info, None)
                .expect("Failed to create image")
        };

        // Allocate memory
        let requirements = unsafe { device.handle().get_image_memory_requirements(image) };
        let allocation = allocator.handle()
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Image",
                requirements,
                location: MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            }).unwrap();

        unsafe {
            device.handle().bind_image_memory(image, allocation.memory(), allocation.offset())
            .expect("Failed to bind image memory")
        }

        let image_view_create_info = vk::ImageViewCreateInfo::default()
            .format(vk::Format::R8G8B8A8_UNORM)
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .components(ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let image_view = unsafe {
            device.handle().create_image_view(&image_view_create_info, None)
                .expect("Failed to create image")
        };

        let sampler_create_info = vk::SamplerCreateInfo::default();

        let sampler = unsafe {
            device.handle().create_sampler(&sampler_create_info, None)
                .expect("Failed to create sampler")
        };

        Image {
            image,
            image_view,
            sampler,
            allocation,
            device_dep: device.inner.clone(),
            allocator_dep: allocator.inner.clone(),
        }
    }
}