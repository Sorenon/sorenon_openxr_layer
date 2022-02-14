use ash::{prelude::VkResult, vk, Device, Instance};

use crate::{ImageFormat, InteropHandle};

lazy_static::lazy_static! {
    static ref VK_FORMATS: bimap::BiHashMap<ImageFormat, vk::Format> = {
        use vk::Format;
        [
            (ImageFormat::Rgba8Unorm, Format::R8G8B8A8_UNORM),
            (ImageFormat::Rgba8UnormSrgb, Format::R8G8B8_SRGB),

            (ImageFormat::Rgb10a2Unorm, Format::A2R10G10B10_UNORM_PACK32),

            (ImageFormat::Rgba16Float, Format::R16G16B16A16_SFLOAT),

            (ImageFormat::Rgba32Float, Format::R32G32B32A32_SFLOAT),

            (ImageFormat::Depth32Float, Format::D32_SFLOAT),
            (ImageFormat::Depth24PlusStencil8, Format::D24_UNORM_S8_UINT),

            //ImageFormat::Depth24FloatPlusStencil8Uint
            (ImageFormat::Depth16Unorm, Format::D16_UNORM),
        ]
        .into_iter()
        .collect::<bimap::BiHashMap<_, _>>()
    };
}

pub fn needed_instance_extensions() -> Vec<*const i8> {
    vec![
        vk::KhrExternalMemoryCapabilitiesFn::name().as_ptr(),
        vk::KhrExternalSemaphoreCapabilitiesFn::name().as_ptr(),
    ]
}

pub fn needed_device_extensions() -> Vec<*const i8> {
    vec![
        vk::KhrExternalMemoryFn::name().as_ptr(),
        #[cfg(target_os = "windows")]
        vk::KhrExternalMemoryWin32Fn::name().as_ptr(),
        #[cfg(target_os = "linux")]
        vk::KhrExternalMemoryFdFn::name().as_ptr(),
    ]
}

pub struct VulkanInterop {
    // instance: Instance,
    // physical_device: vk::PhysicalDevice,
    device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    device: Device,

    #[cfg(target_os = "windows")]
    khr_external_memory: vk::KhrExternalMemoryWin32Fn,

    #[cfg(target_os = "linux")]
    khr_external_memory: vk::KhrExternalMemoryFdFn,
}

impl VulkanInterop {
    pub fn new(instance: &Instance, physical_device: vk::PhysicalDevice, device: &Device) -> Self {
        let device_memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };

        #[cfg(target_os = "windows")]
        let khr_external_memory = unsafe {
            let load_fn = |name: &std::ffi::CStr| {
                std::mem::transmute(instance.get_device_proc_addr(device.handle(), name.as_ptr()))
            };
            vk::KhrExternalMemoryWin32Fn::load(load_fn)
        };

        let khr_external_memory = unsafe {
            let load_fn = |name: &std::ffi::CStr| {
                std::mem::transmute(instance.get_device_proc_addr(device.handle(), name.as_ptr()))
            };
            #[cfg(target_os = "linux")]
            vk::KhrExternalMemoryFdFn::load(load_fn)
        };

        Self {
            device_memory_properties,
            device: device.clone(),
            khr_external_memory,
        }
    }

    pub fn create_external_image(
        &self,
        image_create_info: &crate::ImageCreateInfo,
    ) -> VkResult<vk::Image> {
        let export_info = vk::ExternalMemoryImageCreateInfo {
            #[cfg(target_os = "windows")]
            handle_types: vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32,
            #[cfg(target_os = "linux")]
            handle_types: vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD,
            ..Default::default()
        };

        let create_info = vk::ImageCreateInfo {
            p_next: &export_info as *const _ as _,
            image_type: vk::ImageType::TYPE_2D,
            format: *VK_FORMATS.get_by_left(&image_create_info.format).unwrap(),
            extent: vk::Extent3D {
                width: image_create_info.width,
                height: image_create_info.height,
                depth: 1,
            },
            mip_levels: image_create_info.mip_count,
            array_layers: image_create_info.layers,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        unsafe { self.device.create_image(&create_info, None) }
    }

    pub fn alloc_and_bind_external_image(
        &self,
        image: vk::Image,
    ) -> VkResult<(vk::DeviceMemory, u64)> {
        let texture_memory_req = unsafe { self.device.get_image_memory_requirements(image) };
        let texture_memory_index = self
            .find_memory_type_index(&texture_memory_req, vk::MemoryPropertyFlags::DEVICE_LOCAL)
            .expect("Unable to find suitable memory index for depth image.");

        #[cfg(windows)]
        let export_mem_alloc_info = vk::ExportMemoryAllocateInfo::builder()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32)
            .build();
        #[cfg(target_os = "linux")]
        let export_mem_alloc_info = vk::ExportMemoryAllocateInfo::builder()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD)
            .build();

        let texture_allocate_info = vk::MemoryAllocateInfo {
            p_next: &export_mem_alloc_info as *const _ as _,
            allocation_size: texture_memory_req.size,
            memory_type_index: texture_memory_index,
            ..Default::default()
        };

        unsafe {
            let memory = self.device.allocate_memory(&texture_allocate_info, None)?;
            self.device.bind_image_memory(image, memory, 0)?;
            Ok((memory, texture_memory_req.size))
        }
    }

    fn find_memory_type_index(
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

    pub fn get_external_memory_handle(&self, memory: vk::DeviceMemory) -> VkResult<InteropHandle> {
        #[cfg(target_os = "windows")]
        unsafe {
            let mut handle = std::ptr::null_mut();

            let win32_handle_info = vk::MemoryGetWin32HandleInfoKHR::builder()
                .memory(memory)
                .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32)
                .build();

            self.khr_external_memory
                .get_memory_win32_handle_khr(self.device.handle(), &win32_handle_info, &mut handle)
                .result()?;
            Ok(handle)
        }

        #[cfg(target_os = "linux")]
        unsafe {
            let mut handle = 0;

            let handle_info = vk::MemoryGetFdInfoKHR::builder()
                .memory(memory)
                .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD)
                .build();

            self.khr_external_memory
                .get_memory_fd_khr(self.device.handle(), &handle_info, &mut handle)
                .result()?;

            Ok(handle)
        }
    }
}

impl ImageFormat {
    pub fn to_vk(&self) -> Option<vk::Format> {
        VK_FORMATS.get_by_left(self).copied()
    }

    pub fn from_vk(gl_format: vk::Format) -> Option<Self> {
        VK_FORMATS.get_by_right(&gl_format).copied()
    }
}
