use std::collections::HashMap;
use std::mem::size_of;

use ash::vk;
use glam::{UVec2, UVec3};
use log::error;
use slotmap::DefaultKey;

use crate::graphics::{PipelineConfig, Renderer};
use crate::graphics::renderer::PushConstants;
use crate::vulkan::{CommandBuffer, DescriptorSetLayout, Image};
use crate::vulkan::PipelineErr;

pub enum DispatchConfig
{
    Count( u32, u32, u32 ),
    FullScreen,
}

pub struct Pass {
    pub shader: String,
    pub dispatches: DispatchConfig,
    pub input_resources: Vec<u32>,
    pub output_resources: Vec<u32>,
}

#[derive(Clone)]
pub enum ClearConfig {
    None,
    Color(f32,f32,f32),
}

#[derive(Clone)]
pub struct ImageConfig {
    pub clear: ClearConfig,
}

pub struct DrawConfig {
    pub passes: Vec<Pass>,
    pub images: Vec<ImageConfig>,
}

pub struct ShaderPass {
    pub dispatches: glam::UVec3,
    pub in_images: Vec<u32>,
    pub out_images: Vec<u32>,
    pub pipeline_handle: DefaultKey,
}

pub struct ImageResource {
    pub image: Image,
    pub clear: ClearConfig,
}

/**
 *  Contains all render related structures relating to a config.
 */
pub struct DrawOrchestrator {
    pub compute_descriptor_set_layout: DescriptorSetLayout,
    pub image_resources: Vec<ImageResource>,
    pub passes: Vec<ShaderPass>,
}

impl DrawOrchestrator {
    pub fn new(renderer: &mut Renderer, resolution: UVec2, draw_config: &DrawConfig) -> Result<DrawOrchestrator, PipelineErr> {

        let image_count = draw_config.images.len() as u32;

        // Verify max referred index
        let max_reffered_image = draw_config.passes.iter()
            .map(|p| p.output_resources.iter())
            .flatten().max().unwrap();
        if *max_reffered_image as i32 > image_count as i32 - 1 {
            error!("Image index out of bounds, provide enough image resources");
            panic!("Image index out of bounds, provide enough image resources");
        }

        // Layout
        let layout_bindings = &[
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(image_count)
                .stage_flags(vk::ShaderStageFlags::COMPUTE | vk::ShaderStageFlags::FRAGMENT)
        ];
        let compute_descriptor_set_layout = DescriptorSetLayout::new_push_descriptor(
            &renderer.device,
            layout_bindings
        );

        // Images
        let image_resources = draw_config.images.iter().map(|c| {
            let image = Image::new(
                &renderer.device,
                &mut renderer.allocator,
                resolution.x,
                resolution.y,
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST
            );

            ImageResource {
                image,
                clear: c.clear.clone(),
            }
        }).collect::<Vec<ImageResource>>();

        // Transition images
        let mut image_command_buffer = CommandBuffer::new(&renderer.device, &renderer.command_pool);
        image_command_buffer.begin();
        {
            for image_resource in &image_resources {
                renderer.transition_image(&image_command_buffer, &image_resource.image.handle(), vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL, vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::BOTTOM_OF_PIPE, vk::AccessFlags::empty(), vk::AccessFlags::empty());
            }
        }
        image_command_buffer.end();
        renderer.device.submit_single_time_command(renderer.queue, &image_command_buffer);

        let push_constant_ranges = Vec::from([
            vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
                .offset(0)
                .size(size_of::<PushConstants>() as u32),
        ]);

        let workgroup_size = 32;
        let full_screen_dispatches = UVec3::new(
            (resolution.x as f32 / workgroup_size as f32).ceil() as u32,
            (resolution.y as f32 / workgroup_size as f32).ceil() as u32,
            1
        );

        let mut macros: HashMap<String, String> = HashMap::new();
        macros.insert("NUM_IMAGES".to_string(), image_count.to_string());
        macros.insert("WORKGROUP_SIZE".to_string(), workgroup_size.to_string());

        // Passes
        let passes = draw_config.passes
            .iter()
            .map(|c| {
                let pipeline_handle = renderer.pipeline_store.insert(
                    PipelineConfig {
                        shader_path: c.shader.clone().into(),
                        descriptor_set_layouts: vec![compute_descriptor_set_layout.clone()],
                        push_constant_ranges: push_constant_ranges.clone(),
                        macros: macros.clone()
                    }
                ).unwrap();

                let dispatches = match c.dispatches {
                    DispatchConfig::Count(x, y, z) => {
                        UVec3::new(x, y, z)
                    }
                    DispatchConfig::FullScreen => {
                        full_screen_dispatches
                    }
                };

                Ok(ShaderPass {
                    pipeline_handle,
                    dispatches: dispatches,
                    in_images: c.input_resources.clone(),
                    out_images: c.output_resources.clone(),
                })
            })
            .collect::<Result<Vec<ShaderPass>, PipelineErr>>()?;

        Ok(DrawOrchestrator {
            compute_descriptor_set_layout,
            image_resources,
            passes
        })
    }
}