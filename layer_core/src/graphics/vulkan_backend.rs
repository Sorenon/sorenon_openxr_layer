use core::slice;
use std::sync::Arc;

use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use openxr::sys as xr;

use crate::wrappers::{instance::InnerInstance, swapchain::SwapchainBackend};

use super::vulkan::VkBackend;

pub struct SwapchainBackendVulkan {
    vk_backend: Arc<VkBackend>,
    images: Vec<vk::Image>,
    memory: Vec<(vk::DeviceMemory, u64)>,
    runtime_images: Vec<vk::Image>,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    image_views: Vec<vk::ImageView>,
    runtime_image_views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    command_buffers: Vec<vk::CommandBuffer>,
    descriptor_pool: vk::DescriptorPool,
}

impl SwapchainBackendVulkan {
    pub fn load(
        swapchain: xr::Swapchain,
        inner: &InnerInstance,
        vk_backend: Arc<VkBackend>,
        image_info: &graphics_interop::ImageCreateInfo,
    ) -> Self {
        let runtime_images = unsafe {
            crate::interceptors::call_enumerate(
                swapchain,
                std::mem::transmute(inner.core.enumerate_swapchain_images),
                xr::SwapchainImageVulkanKHR::out(std::ptr::null_mut()).assume_init(),
            )
        }
        .unwrap()
        .into_iter()
        .map(|image| vk::Image::from_raw(image.image))
        .collect::<Vec<_>>();

        let mut memory = Vec::with_capacity(runtime_images.len());
        let mut images = Vec::with_capacity(runtime_images.len());

        let cb_memory_barrier = unsafe {
            *vk_backend
                .device
                .allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                    command_pool: vk_backend.command_pool,
                    level: vk::CommandBufferLevel::PRIMARY,
                    command_buffer_count: 1,
                    ..Default::default()
                })
                .unwrap()
                .first()
                .unwrap()
        };
        unsafe {
            vk_backend
                .device
                .begin_command_buffer(
                    cb_memory_barrier,
                    &vk::CommandBufferBeginInfo {
                        flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                        ..Default::default()
                    },
                )
                .unwrap();
        }

        for i in 0..runtime_images.len() {
            images.push(
                vk_backend
                    .interop
                    .create_external_image(image_info)
                    .unwrap(),
            );
            memory.push(
                vk_backend
                    .interop
                    .alloc_and_bind_external_image(images[i])
                    .unwrap(),
            );

            unsafe {
                vk_backend.device.cmd_pipeline_barrier(
                    cb_memory_barrier,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[vk::ImageMemoryBarrier {
                        new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        image: images[i],
                        subresource_range: vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            level_count: 1,
                            layer_count: image_info.layers,
                            ..Default::default()
                        },
                        ..Default::default()
                    }],
                );
            }
        }

        unsafe {
            vk_backend
                .device
                .end_command_buffer(cb_memory_barrier)
                .unwrap();
            vk_backend
                .device
                .queue_submit(
                    vk_backend.graphics_queue,
                    &[vk::SubmitInfo::builder()
                        .command_buffers(std::slice::from_ref(&cb_memory_barrier))
                        .build()],
                    vk::Fence::null(),
                )
                .unwrap();
            vk_backend
                .device
                .queue_wait_idle(vk_backend.graphics_queue)
                .unwrap();
            vk_backend
                .device
                .free_command_buffers(vk_backend.command_pool, &[cb_memory_barrier]);
        }

        let (pipeline_layout, render_pass, pipeline) = unsafe {
            vk_backend.create_graphics_pipeline(
                image_info.width,
                image_info.height,
                image_info.format.to_vk().unwrap(),
                vk::SampleCountFlags::TYPE_1,
                std::slice::from_ref(&vk_backend.descriptor_set_layout),
            )
        };

        let image_views = images
            .iter()
            .map(|&image| {
                vk_backend.create_image_view(
                    image,
                    image_info.format.to_vk().unwrap(),
                    image_info.layers,
                )
            })
            .collect::<VkResult<Vec<_>>>()
            .unwrap();

        let runtime_image_views = runtime_images
            .iter()
            .map(|&image| {
                vk_backend.create_image_view(
                    image,
                    image_info.format.to_vk().unwrap(),
                    image_info.layers,
                )
            })
            .collect::<VkResult<Vec<_>>>()
            .unwrap();

        let framebuffers = runtime_image_views
            .iter()
            .map(|image_view| {
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(std::slice::from_ref(image_view))
                    .width(image_info.width)
                    .height(image_info.height)
                    .layers(image_info.layers);
                unsafe { vk_backend.device.create_framebuffer(&create_info, None) }
            })
            .collect::<VkResult<Vec<_>>>()
            .unwrap();

        let command_buffers = unsafe {
            vk_backend.device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(vk_backend.command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(framebuffers.len() as u32),
            )
        }
        .unwrap();

        let descriptor_pool = unsafe {
            let descriptor_size = vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: image_views.len() as u32,
            };
            let create_info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(std::slice::from_ref(&descriptor_size))
                .max_sets(image_views.len() as u32);

            vk_backend.device.create_descriptor_pool(&create_info, None)
        }
        .unwrap();

        let descriptor_sets = {
            unsafe {
                vk_backend.device.allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::builder()
                        .descriptor_pool(descriptor_pool)
                        .set_layouts(&vec![vk_backend.descriptor_set_layout; image_views.len()]),
                )
            }
        }
        .unwrap();

        for (&image_view, &set) in image_views.iter().zip(descriptor_sets.iter()) {
            let image_info = vk::DescriptorImageInfo {
                sampler: vk_backend.nearest_sampler,
                image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            };
            let descriptor_write = vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&image_info));

            unsafe {
                vk_backend
                    .device
                    .update_descriptor_sets(&[*descriptor_write], &[]);
            }
        }

        for (i, &command_buffer) in command_buffers.iter().enumerate() {
            let framebuffer = framebuffers[i];
            let set = descriptor_sets[i];
            unsafe {
                vk_backend
                    .device
                    .begin_command_buffer(command_buffer, &vk::CommandBufferBeginInfo::default())
                    .unwrap();
                //On my Manjaro Linux install with proprietary Nvidia drivers this causes the interop image to be entirely black
                //This issue has not been tested on any other machines so its disabled by default
                // #[cfg(windows)]
                // vk_backend.device.cmd_pipeline_barrier(
                //     command_buffer,
                //     vk::PipelineStageFlags::TOP_OF_PIPE,
                //     vk::PipelineStageFlags::FRAGMENT_SHADER,
                //     vk::DependencyFlags::empty(),
                //     &[],
                //     &[],
                //     &[vk::ImageMemoryBarrier {
                //         dst_access_mask: vk::AccessFlags::SHADER_READ,
                //         new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                //         image: images[i],
                //         subresource_range: vk::ImageSubresourceRange {
                //             aspect_mask: vk::ImageAspectFlags::COLOR,
                //             level_count: 1,
                //             layer_count: 1,
                //             ..Default::default()
                //         },
                //         ..Default::default()
                //     }],
                // );
                vk_backend.device.cmd_begin_render_pass(
                    command_buffer,
                    &vk::RenderPassBeginInfo::builder()
                        .render_pass(render_pass)
                        .framebuffer(framebuffer)
                        .render_area(vk::Rect2D {
                            offset: vk::Offset2D { x: 0, y: 0 },
                            extent: vk::Extent2D {
                                width: image_info.width,
                                height: image_info.height,
                            },
                        }),
                    vk::SubpassContents::INLINE,
                );
                vk_backend.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline,
                );
                vk_backend.device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline_layout,
                    0,
                    std::slice::from_ref(&set),
                    &[],
                );
                vk_backend.device.cmd_draw(command_buffer, 3, image_info.layers, 0, 0);
                vk_backend.device.cmd_end_render_pass(command_buffer);
                vk_backend
                    .device
                    .end_command_buffer(command_buffer)
                    .unwrap();
            }
        }

        Self {
            vk_backend,
            images,
            memory,
            runtime_images,
            pipeline_layout,
            render_pass,
            pipeline,
            image_views,
            runtime_image_views,
            framebuffers,
            command_buffers,
            descriptor_pool,
        }
    }
}

impl SwapchainBackend for SwapchainBackendVulkan {
    fn get_external_memory_handles(&self) -> Vec<(graphics_interop::InteropHandle, u64)> {
        self.memory
            .iter()
            .map(|&(mem, size)| {
                (
                    self.vk_backend
                        .interop
                        .get_external_memory_handle(mem)
                        .unwrap(),
                    size,
                )
            })
            .collect()
    }

    fn release_image(&self, index: usize) {
        unsafe {
            self.vk_backend
                .device
                .queue_submit(
                    self.vk_backend.graphics_queue,
                    slice::from_ref(
                        &vk::SubmitInfo::builder()
                            .command_buffers(slice::from_ref(&self.command_buffers[index])),
                    ),
                    vk::Fence::null(),
                )
                .unwrap();
            self.vk_backend
                .device
                .queue_wait_idle(self.vk_backend.graphics_queue)
                .unwrap();
        }
    }

    fn destroy(&self) {
        unsafe {
            let device = &self.vk_backend.device;
            for &image in &self.images {
                device.destroy_image(image, None)
            }
            for &(mem, _) in &self.memory {
                device.free_memory(mem, None);
            }
            for &image in &self.runtime_images {
                device.destroy_image(image, None)
            }
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_render_pass(self.render_pass, None);
            for &view in &self.image_views {
                device.destroy_image_view(view, None)
            }
            for &view in &self.runtime_image_views {
                device.destroy_image_view(view, None)
            }
            for &framebuffer in &self.framebuffers {
                device.destroy_framebuffer(framebuffer, None)
            }
            device.free_command_buffers(self.vk_backend.command_pool, &self.command_buffers[..]);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}
