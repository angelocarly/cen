use cen::graphics::pipeline_store::{PipelineConfig, PipelineKey};
use ash::vk;
use ash::vk::WriteDescriptorSet;
use cen::app::App;
use cen::app::app::AppConfig;
use cen::graphics::Renderer;
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::{CommandBuffer, DescriptorSetLayout, Image};

struct ComputeRender {
    image: Image,
    descriptorset: DescriptorSetLayout,
    pipeline: PipelineKey,
}

impl ComputeRender {
    fn new(renderer: &mut Renderer) -> Self {

        // Image
        let image = Image::new(
            &renderer.device,
            &mut renderer.allocator,
            renderer.swapchain.get_extent().width,
            renderer.swapchain.get_extent().height,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST
        );

        // Transition image
        let mut image_command_buffer = CommandBuffer::new(&renderer.device, &renderer.command_pool);
        image_command_buffer.begin();
        {
            renderer.transition_image(&image_command_buffer, image.handle(), vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL, vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::BOTTOM_OF_PIPE, vk::AccessFlags::empty(), vk::AccessFlags::empty());
        }
        image_command_buffer.end();
        renderer.device.submit_single_time_command(renderer.queue, &image_command_buffer);

        // Layout
        let layout_bindings = &[
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE ),
        ];
        let descriptorset = DescriptorSetLayout::new_push_descriptor(
            &renderer.device,
            layout_bindings
        );

        // Pipeline
        let pipeline = renderer.pipeline_store().insert(PipelineConfig {
            shader_path: "examples/compute/shader.comp".into(),
            descriptor_set_layouts: vec![
                descriptorset.clone(),
            ],
            push_constant_ranges: vec![],
            macros: Default::default(),
        }).expect("Failed to create pipeline");

        Self {
            image,
            descriptorset,
            pipeline
        }
    }
}

impl RenderComponent for ComputeRender {
    fn render(&self, renderer: &mut Renderer, command_buffer: &mut CommandBuffer, swapchain_image: &vk::Image) {
        // Render
        let compute = renderer.pipeline_store().get(self.pipeline).unwrap();
        command_buffer.bind_pipeline(&compute);

        let bindings = [self.image.binding(vk::ImageLayout::GENERAL)];

        let write_descriptor_set = WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&bindings);

        command_buffer.bind_push_descriptor(
            &compute,
            0,
            &[write_descriptor_set]
        );
        command_buffer.dispatch(500, 500, 1 );

        // Transition the render to a source
        renderer.transition_image(
            &command_buffer,
            &self.image.handle(),
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::SHADER_WRITE,
            vk::AccessFlags::TRANSFER_READ
        );

        // Transition the swapchain image
        renderer.transition_image(
            &command_buffer,
            &swapchain_image,
            vk::ImageLayout::PRESENT_SRC_KHR,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::NONE,
            vk::AccessFlags::TRANSFER_WRITE
        );

        // Copy to the swapchain
        unsafe {

            renderer.device.handle().cmd_clear_color_image(
                command_buffer.handle(),
                *swapchain_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0]
                },
                &[vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }]
            );

            // Use a blit, as a copy doesn't synchronize properly to the swapchain on MoltenVK
            renderer.device.handle().cmd_blit_image(
                command_buffer.handle(),
                *self.image.handle(),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                *swapchain_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[vk::ImageBlit::default()
                    .src_offsets([
                        vk::Offset3D::default(),
                        vk::Offset3D::default().x(self.image.width as i32).y(self.image.height as i32).z(1)
                    ])
                    .dst_offsets([
                        vk::Offset3D::default(),
                        vk::Offset3D::default().x(self.image.width as i32).y(self.image.height as i32).z(1)
                    ])
                    .src_subresource(
                        vk::ImageSubresourceLayers::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_array_layer(0)
                            .layer_count(1)
                            .mip_level(0)
                    )
                    .dst_subresource(
                        vk::ImageSubresourceLayers::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_array_layer(0)
                            .layer_count(1)
                            .mip_level(0)
                    )
                ],
                vk::Filter::NEAREST,
            );
        }

        // Transfer back to default states
        renderer.transition_image(
            &command_buffer,
            &swapchain_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::NONE
        );

        // Transition the render image back
        renderer.transition_image(
            &command_buffer,
            &self.image.handle(),
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::ImageLayout::GENERAL,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::NONE
        );

    }
}

fn main() {
    let mut app = App::new(AppConfig {
        width: 1000,
        height: 1000,
        vsync: false,
        log_fps: false,
    });

    let compute_example = ComputeRender::new(app.renderer());

    app.run(Box::new(compute_example));
}