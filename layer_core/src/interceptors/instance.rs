use std::sync::Arc;

use ash::vk::Handle;
use log::{debug, error, info};
use openxr::{
    sys::{self as xr, pfn},
    Result,
};

use crate::{
    graphics::{opengl::frontend::OpenGLFrontend, vulkan},
    wrappers::{
        instance::{GraphicsEnableFlags, InstanceWrapper, SystemMeta},
        session::{SessionGraphics, SessionWrapper},
        XrHandle,
    },
    ToResult,
};

pub(super) unsafe fn get_instance_interceptors(name: &str) -> Option<pfn::VoidFunction> {
    use std::mem::transmute;
    use xr::pfn::*;
    Some(match name {
        "xrGetSystem" => transmute(xr_get_system as GetSystem),
        "xrGetOpenGLGraphicsRequirementsKHR" => {
            transmute(xr_get_opengl_graphics_requirements_khr as GetOpenGLGraphicsRequirementsKHR)
        }
        "xrCreateSession" => transmute(xr_create_session as CreateSession),
        _ => return None,
    })
}

unsafe extern "system" fn xr_get_system(
    instance: xr::Instance,
    get_info: *const xr::SystemGetInfo,
    system_id: *mut xr::SystemId,
) -> xr::Result {
    instance.run(|instance| get_system(instance, &*get_info, &mut *system_id))
}

unsafe extern "system" fn xr_get_opengl_graphics_requirements_khr(
    instance: xr::Instance,
    system_id: xr::SystemId,
    graphics_requirements: *mut xr::GraphicsRequirementsOpenGLKHR,
) -> xr::Result {
    instance.run(|instance| {
        match instance.systems.get_mut(&system_id) {
            Some(mut system_meta) => {
                system_meta.requirements_called |= GraphicsEnableFlags::OPENGL_GL;
            }
            None => return Err(xr::Result::ERROR_SYSTEM_INVALID),
        }

        (*graphics_requirements).max_api_version_supported = openxr::Version::new(4, 6, 0);
        (*graphics_requirements).min_api_version_supported = openxr::Version::new(4, 5, 0);

        Ok(xr::Result::SUCCESS)
    })
}

unsafe extern "system" fn xr_create_session(
    instance: xr::Instance,
    create_info: *const xr::SessionCreateInfo,
    session: *mut xr::Session,
) -> xr::Result {
    instance.run(|instance| create_session(instance, &*create_info, &mut *session))
}

fn get_system(
    instance: &InstanceWrapper,
    get_info: &xr::SystemGetInfo,
    system_id: &mut xr::SystemId,
) -> Result<xr::Result> {
    let success = unsafe { (instance.inner.core.get_system)(instance.handle, get_info, system_id) }
        .result()?;

    instance.systems.insert(
        *system_id,
        SystemMeta {
            form_factor: get_info.form_factor,
            requirements_called: GraphicsEnableFlags::empty(),
        },
    );

    debug!(
        "Get system called: form_factor={:?}, id={}",
        get_info.form_factor,
        system_id.into_raw()
    );

    Ok(success)
}

fn create_session(
    instance: &Arc<InstanceWrapper>,
    create_info: &xr::SessionCreateInfo,
    session: &mut xr::Session,
) -> Result<xr::Result> {
    let mut needs_compat = false;

    let opengl_override = true;

    unsafe {
        if create_info.next != std::ptr::null() {
            let next: &xr::BaseInStructure = std::mem::transmute(create_info.next);

            if next.next != std::ptr::null() {
                todo!();
            }

            needs_compat = match next.ty {
                xr::StructureType::GRAPHICS_BINDING_D3D11_KHR => false,
                xr::StructureType::GRAPHICS_BINDING_D3D12_KHR => false,
                xr::StructureType::GRAPHICS_BINDING_EGL_MNDX => opengl_override,
                xr::StructureType::GRAPHICS_BINDING_OPENGL_WIN32_KHR => opengl_override,
                xr::StructureType::GRAPHICS_BINDING_OPENGL_XLIB_KHR => opengl_override,
                xr::StructureType::GRAPHICS_BINDING_OPENGL_XCB_KHR => opengl_override,
                xr::StructureType::GRAPHICS_BINDING_OPENGL_WAYLAND_KHR => opengl_override,
                xr::StructureType::GRAPHICS_BINDING_OPENGL_ES_ANDROID_KHR => opengl_override,
                xr::StructureType::GRAPHICS_BINDING_VULKAN_KHR => false,
                _ => false,
            };
        }
    }

    let session_wrapper = if needs_compat {
        let opengl_context = unsafe {
            use crate::graphics::opengl::*;
            let next: &xr::BaseInStructure = std::mem::transmute(create_info.next);
            match next.ty {
                xr::StructureType::GRAPHICS_BINDING_EGL_MNDX => todo!(),
                xr::StructureType::GRAPHICS_BINDING_OPENGL_WIN32_KHR => {
                    #[cfg(windows)]
                    {
                        let binding: &xr::GraphicsBindingOpenGLWin32KHR =
                            unsafe { std::mem::transmute(create_info.next) };
                        GLContext::Wgl(platform::windows::WGL::load(binding.h_dc, binding.h_glrc))
                    }
                    #[cfg(target_os = "linux")]
                    todo!()
                }
                xr::StructureType::GRAPHICS_BINDING_OPENGL_XLIB_KHR => {
                    #[cfg(target_os = "linux")]
                    {
                        let binding: &xr::GraphicsBindingOpenGLXlibKHR =
                            std::mem::transmute(create_info.next);
                        GLContext::X11(platform::linux::X11 {
                            x_display: binding.x_display as _,
                            visualid: binding.visualid,
                            glx_fb_config: binding.glx_fb_config,
                            glx_drawable: binding.glx_drawable,
                            glx_context: binding.glx_context,
                        })
                    }
                    #[cfg(windows)]
                    todo!()
                }
                xr::StructureType::GRAPHICS_BINDING_OPENGL_XCB_KHR => todo!(),
                xr::StructureType::GRAPHICS_BINDING_OPENGL_WAYLAND_KHR => todo!(),
                xr::StructureType::GRAPHICS_BINDING_OPENGL_ES_ANDROID_KHR => todo!(),
                _ => unreachable!(),
            }
        };
        if !instance
            .systems
            .get(&create_info.system_id)
            .ok_or(xr::Result::ERROR_SYSTEM_INVALID)?
            .requirements_called
            .contains(GraphicsEnableFlags::OPENGL_GL)
        {
            return Err(xr::Result::ERROR_GRAPHICS_REQUIREMENTS_CALL_MISSING);
        }

        opengl_context.make_current();

        let vk_backend = unsafe {
            vulkan::VkBackend::new_openxr(&instance, create_info.system_id).map_err(|_| {
                error!("Backend vulkan base creation failed");
                xr::Result::ERROR_RUNTIME_FAILURE
            })?
        };

        debug!("Created vulkan backend successfully!");

        let vulkan = xr::GraphicsBindingVulkanKHR {
            ty: xr::GraphicsBindingVulkanKHR::TYPE,
            next: std::ptr::null(),
            instance: vk_backend.instance.handle().as_raw() as _,
            physical_device: vk_backend.physical_device.as_raw() as _,
            device: vk_backend.device.handle().as_raw() as _,
            queue_family_index: vk_backend.graphics_queue_family,
            queue_index: 0,
        };

        let create_info2 = xr::SessionCreateInfo {
            ty: xr::SessionCreateInfo::TYPE,
            next: &vulkan as *const _ as _,
            create_flags: xr::SessionCreateFlags::EMPTY,
            system_id: create_info.system_id,
        };

        unsafe { (instance.inner.core.create_session)(instance.handle, &create_info2, session) }
            .result()?;

        let swapchain_formats = unsafe {
            super::call_enumerate(*session, instance.inner.core.enumerate_swapchain_formats, 0)?
                .iter()
                .filter_map(|backend_format| {
                    let vulkan_format = ash::vk::Format::from_raw(*backend_format as i32);
                    graphics_interop::ImageFormat::from_vk(vulkan_format)
                        .map(|format| {
                            log::info!("{:?}", format);
                            format.to_gl()
                        })
                        .flatten()
                        .map(|f| f as i64)
                })
                .collect::<Vec<_>>()
        };

        Arc::new(SessionWrapper {
            handle: *session,
            instance: Arc::downgrade(instance),
            inner: instance.inner.clone(),
            graphics: SessionGraphics::Compat {
                frontend: Arc::new(OpenGLFrontend::load(opengl_context)),
                backend: Arc::new(vk_backend),
                swapchain_formats,
            },
            swapchains: Default::default(),
        })
    } else {
        unsafe { (instance.inner.core.create_session)(instance.handle, create_info, session) }
            .result()?;
        Arc::new(SessionWrapper {
            handle: *session,
            instance: Arc::downgrade(instance),
            inner: instance.inner.clone(),
            graphics: SessionGraphics::Direct,
            swapchains: Default::default(),
        })
    };

    *session = session_wrapper.handle;
    xr::Session::all_wrappers().insert(*session, session_wrapper.clone());
    instance.sessions.insert(*session, session_wrapper);
    info!("Session created: {:?}", *session);

    Ok(xr::Result::SUCCESS)
}
