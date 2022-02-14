use core::slice;
use std::{borrow::Cow, ffi::CStr, io::Cursor, os::raw::c_char};

use ash::{
    extensions::ext::DebugUtils,
    prelude::VkResult,
    vk::{self, DebugUtilsMessengerEXT, Handle},
    Device, Entry, Instance,
};
use graphics_interop::apis::vulkan::VulkanInterop;
use log::error;
use openxr::sys as xr;

use crate::{wrappers::instance::InstanceWrapper, ToResult};

pub struct VkBackend {
    pub entry: Entry,
    pub instance: Instance,
    pub device: Device,
    pub debug_utils: DebugUtils,
    pub debug_messenger: vk::DebugUtilsMessengerEXT,

    pub physical_device: vk::PhysicalDevice,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub graphics_queue_family: u32,
    pub graphics_queue: vk::Queue,

    pub command_pool: vk::CommandPool,
    pub nearest_sampler: vk::Sampler,
    pub descriptor_set_layout: vk::DescriptorSetLayout,

    pub interop: VulkanInterop,
}

impl VkBackend {
    pub unsafe fn new_openxr(
        xr_instance: &InstanceWrapper,
        system_id: xr::SystemId,
    ) -> Result<VkBackend, ()> {
        let entry = Entry::load().unwrap();
        let exts = &xr_instance.inner.exts;
        let xr_instance = xr_instance.handle;

        let _requirements = if let Some(vulkan) = exts.khr_vulkan_enable2 {
            let mut reqs =
                xr::GraphicsRequirementsVulkanKHR::out(std::ptr::null_mut()).assume_init();
            let result =
                (vulkan.get_vulkan_graphics_requirements2)(xr_instance, system_id, &mut reqs);
            if result.result().is_err() {
                error!("get_vulkan_graphics_requirements2 returned: {}", result);
                return Err(());
            }
            reqs
        } else {
            todo!()
        };

        //TODO actually check requirements

        let layer_names = [CStr::from_bytes_with_nul_unchecked(
            b"VK_LAYER_KHRONOS_validation\0",
        )];
        let layers_names_raw: Vec<*const c_char> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let instance_extensions = [DebugUtils::name().as_ptr()];

        let app_info = vk::ApplicationInfo::builder()
            .application_name(CStr::from_bytes_with_nul_unchecked(b"SorenonOpenXRLayer\0"))
            .application_version(0)
            .api_version(vk::make_api_version(0, 1, 1, 0));

        let instance_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&instance_extensions);

        // let instance_info = if option_env!("SORENON_LAYER_VK_VALIDATION").is_some() {
        //     instance_info.enabled_layer_names(&layers_names_raw)
        // } else {
        //     instance_info
        // };

        let vk_instance = if let Some(vulkan) = exts.khr_vulkan_enable2 {
            let mut vk_instance = vk::Instance::null();
            let mut vk_result = vk::Result::default();

            let xr_result = (vulkan.create_vulkan_instance)(
                xr_instance,
                &xr::VulkanInstanceCreateInfoKHR {
                    ty: xr::VulkanInstanceCreateInfoKHR::TYPE,
                    next: std::ptr::null_mut(),
                    system_id,
                    create_flags: xr::VulkanInstanceCreateFlagsKHR::EMPTY,
                    pfn_get_instance_proc_addr: Some(std::mem::transmute(
                        entry.static_fn().get_instance_proc_addr,
                    )),
                    vulkan_create_info: &instance_info as *const _ as _,
                    vulkan_allocator: std::ptr::null(),
                },
                &mut vk_instance as *mut _ as _,
                &mut vk_result as *mut _ as _,
            );

            if xr_result.result().is_err() {
                error!("OpenXR error creating vulkan instance: {}", xr_result);
                return Err(());
            } else if vk_result.result().is_err() {
                error!("Vulkan error creating vulkan instance: {}", vk_result);
                return Err(());
            }

            Instance::load(entry.static_fn(), vk_instance)
        } else {
            todo!()
        };

        let (debug_utils, debug_messenger) = create_debug_callback(&entry, &vk_instance).unwrap();

        let physical_device = if let Some(vulkan) = exts.khr_vulkan_enable2 {
            let mut physical_device = vk::PhysicalDevice::null();
            let result = (vulkan.get_vulkan_graphics_device2)(
                xr_instance,
                &xr::VulkanGraphicsDeviceGetInfoKHR {
                    ty: xr::VulkanGraphicsDeviceGetInfoKHR::TYPE,
                    next: std::ptr::null(),
                    system_id,
                    vulkan_instance: vk_instance.handle().as_raw() as _,
                },
                &mut physical_device as *mut _ as _,
            );
            if result.result().is_err() {
                error!("OpenXR error getting physical device: {}", result);
                return Err(());
            }
            physical_device
        } else {
            todo!()
        };

        let device_memory_properties =
            vk_instance.get_physical_device_memory_properties(physical_device);

        let graphics_queue_family = vk_instance
            .get_physical_device_queue_family_properties(physical_device)
            .into_iter()
            .enumerate()
            .find_map(|(queue_family_index, info)| {
                if info.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    Some(queue_family_index as u32)
                } else {
                    None
                }
            })
            .expect("Vulkan device has no graphics queue");

        let mut device_extension_names = graphics_interop::apis::vulkan::needed_device_extensions();
        device_extension_names.push(vk::ExtShaderViewportIndexLayerFn::name().as_ptr());

        let queue_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_family)
            .queue_priorities(&[1.0]);

        let features = vk::PhysicalDeviceFeatures {
            multi_viewport: vk::TRUE,
            ..Default::default()
        };

        let device_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(slice::from_ref(&queue_info))
            .enabled_extension_names(&device_extension_names[..])
            .enabled_features(&features);

        let device = if let Some(vulkan) = exts.khr_vulkan_enable2 {
            let mut device = vk::Device::null();
            let mut vk_result = vk::Result::default();

            let xr_result = (vulkan.create_vulkan_device)(
                xr_instance,
                &xr::VulkanDeviceCreateInfoKHR {
                    ty: xr::VulkanDeviceCreateInfoKHR::TYPE,
                    next: std::ptr::null_mut(),
                    system_id,
                    create_flags: xr::VulkanDeviceCreateFlagsKHR::EMPTY,
                    pfn_get_instance_proc_addr: std::mem::transmute(
                        entry.static_fn().get_instance_proc_addr,
                    ),
                    vulkan_physical_device: physical_device.as_raw() as _,
                    vulkan_create_info: &device_info as *const _ as _,
                    vulkan_allocator: std::ptr::null_mut(),
                },
                &mut device as *mut _ as _,
                &mut vk_result as *mut _ as _,
            );

            if xr_result.result().is_err() {
                error!("OpenXR error creating vulkan device: {}", xr_result);
                return Err(());
            } else if vk_result.result().is_err() {
                error!("Vulkan error creating vulkan device: {}", vk_result);
                return Err(());
            }

            Device::load(vk_instance.fp_v1_0(), device)
        } else {
            todo!()
        };

        let graphics_queue = device.get_device_queue(graphics_queue_family, 0);
        let command_pool = create_command_pool(&device, graphics_queue_family).unwrap();

        let nearest_sampler = {
            let create_info = vk::SamplerCreateInfo::builder()
                    .mag_filter(vk::Filter::NEAREST)
                    .min_filter(vk::Filter::NEAREST)
                    .address_mode_u(vk::SamplerAddressMode::REPEAT)
                    .address_mode_v(vk::SamplerAddressMode::REPEAT)
                    .address_mode_w(vk::SamplerAddressMode::REPEAT)
                    .unnormalized_coordinates(false)//TODO research whether this is optimal
                    .compare_enable(false)
                    .compare_op(vk::CompareOp::ALWAYS)
                    .mipmap_mode(vk::SamplerMipmapMode::NEAREST) //TODO figure out if there is a performant way to copy mip levels
                    ;
            device.create_sampler(&create_info, None)
        }
        .unwrap();

        let descriptor_set_layout = {
            let sampler_layout_binding = vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            };
            let layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(slice::from_ref(&sampler_layout_binding));
            device.create_descriptor_set_layout(&layout_info, None)
        }
        .unwrap();

        let interop = VulkanInterop::new(&vk_instance, physical_device, &device);

        Ok(VkBackend {
            entry,
            instance: vk_instance,
            device,
            debug_utils,
            debug_messenger,
            physical_device,
            device_memory_properties,
            graphics_queue_family,
            graphics_queue,
            command_pool,
            nearest_sampler,
            descriptor_set_layout,
            interop,
        })
    }

    pub fn find_memorytype_index(
        &self,
        memory_req: &vk::MemoryRequirements,
        flags: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        self.device_memory_properties.memory_types
            [..self.device_memory_properties.memory_type_count as _]
            .iter()
            .enumerate()
            .find(|(index, memory_type)| {
                (1 << index) & memory_req.memory_type_bits != 0
                    && memory_type.property_flags & flags == flags
            })
            .map(|(index, _memory_type)| index as _)
    }

    pub fn create_image_view(
        &self,
        image: vk::Image,
        format: vk::Format,
        layers: u32,
    ) -> VkResult<vk::ImageView> {
        let create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
            .format(format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: layers,
            });
        unsafe { self.device.create_image_view(&create_info, None) }
    }

    pub unsafe fn create_graphics_pipeline(
        &self,
        width: u32,
        height: u32,
        format: vk::Format,
        sample_count: vk::SampleCountFlags,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
    ) -> (vk::PipelineLayout, vk::RenderPass, vk::Pipeline) {
        let device = &self.device;
        let vert_shader = create_shader_module(device, VERTEX).unwrap();
        let frag_shader = create_shader_module(device, FRAGMENT).unwrap();

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_shader)
                .name(CStr::from_bytes_with_nul_unchecked(b"main\0"))
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_shader)
                .name(CStr::from_bytes_with_nul_unchecked(b"main\0"))
                .build(),
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default();

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let viewport = vk::Viewport {
            x: 0.,
            y: 0.,
            width: width as f32,
            height: height as f32,
            min_depth: 0.,
            max_depth: 1.,
        };

        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width, height },
        };

        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(slice::from_ref(&viewport))
            .scissors(slice::from_ref(&scissor))
            .build();

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::builder()
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .build()];

        let color_blending =
            vk::PipelineColorBlendStateCreateInfo::builder().attachments(&color_blend_attachments);

        let layout = device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder().set_layouts(descriptor_set_layouts),
                None,
            )
            .unwrap();

        let render_pass = create_render_pass(device, format, sample_count).unwrap();

        //TODO VK_PIPELINE_CREATE_DERIVATIVE_BIT
        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .layout(layout)
            .render_pass(render_pass)
            .subpass(0)
            .build();

        let pipeline = *device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                slice::from_ref(&pipeline_info),
                None,
            )
            .unwrap()
            .first()
            .unwrap();

        device.destroy_shader_module(vert_shader, None);
        device.destroy_shader_module(frag_shader, None);

        (layout, render_pass, pipeline)
    }
}

impl Drop for VkBackend {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.debug_utils
                .destroy_debug_utils_messenger(self.debug_messenger, None);
            self.instance.destroy_instance(None);
        }
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number: i32 = callback_data.message_id_number as i32;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{:?}:\n{:?} [{} ({})] : {}\n",
        message_severity,
        message_type,
        message_id_name,
        &message_id_number.to_string(),
        message,
    );

    vk::FALSE
}

unsafe fn create_debug_callback(
    entry: &Entry,
    instance: &Instance,
) -> VkResult<(DebugUtils, DebugUtilsMessengerEXT)> {
    let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(vulkan_debug_callback));

    let debug_utils_loader = DebugUtils::new(entry, instance);
    let messenger = debug_utils_loader.create_debug_utils_messenger(&debug_info, None)?;
    Ok((debug_utils_loader, messenger))
}

unsafe fn create_command_pool(device: &Device, queue_family: u32) -> VkResult<vk::CommandPool> {
    let pool_create_info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_family);

    device.create_command_pool(&pool_create_info, None)
}

const VERTEX: &[u8] = include_bytes!("../../../shaders/vert.spv");
const FRAGMENT: &[u8] = include_bytes!("../../../shaders/frag.spv");

unsafe fn create_shader_module(device: &Device, code_bytes: &[u8]) -> VkResult<vk::ShaderModule> {
    let shader_code = ash::util::read_spv(&mut Cursor::new(code_bytes)).unwrap();

    let create_info = vk::ShaderModuleCreateInfo::builder().code(&shader_code);
    device.create_shader_module(&create_info, None)
}

unsafe fn create_render_pass(
    device: &Device,
    format: vk::Format,
    sample_count: vk::SampleCountFlags,
) -> VkResult<vk::RenderPass> {
    let color_attachment = vk::AttachmentDescription::builder()
        .format(format)
        .samples(sample_count)
        .load_op(vk::AttachmentLoadOp::DONT_CARE)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let attachment_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(slice::from_ref(&attachment_ref));

    let render_pass_info = vk::RenderPassCreateInfo::builder()
        .attachments(slice::from_ref(&color_attachment))
        .subpasses(slice::from_ref(&subpass));

    device.create_render_pass(&render_pass_info, None)
}
