use std::sync::Arc;

use graphics_interop::ImageFormat;
use log::info;
use openxr::sys as xr;
use openxr::Result;

use crate::graphics::vulkan_backend::SwapchainBackendVulkan;
use crate::wrappers::swapchain::SwapchainBackend;
use crate::wrappers::swapchain::SwapchainGraphics;
use crate::wrappers::swapchain::SwapchainWrapper;
use crate::{
    wrappers::{
        session::{SessionGraphics, SessionWrapper},
        XrHandle,
    },
    ToResult,
};

pub(super) unsafe fn get_session_interceptors(name: &str) -> Option<xr::pfn::VoidFunction> {
    use std::mem::transmute;
    use xr::pfn::*;
    Some(match name {
        "xrEnumerateSwapchainFormats" => {
            transmute(xr_enumerate_swapchain_formats as EnumerateSwapchainFormats)
        }
        "xrCreateSwapchain" => transmute(xr_create_swapchain as CreateSwapchain),
        // "xrEndFrame" => transmute(xr_end_frame as EndFrame),
        _ => return None,
    })
}

pub(crate) unsafe extern "system" fn xr_enumerate_swapchain_formats(
    session: xr::Session,
    format_capacity_input: u32,
    format_count_output: *mut u32,
    formats: *mut i64,
) -> xr::Result {
    session.run(|session| {
        if let SessionGraphics::Compat {
            swapchain_formats, ..
        } = &session.graphics
        {
            super::enumerate(
                format_capacity_input,
                format_count_output,
                formats,
                &swapchain_formats,
            )
        } else {
            (session.inner.core.enumerate_swapchain_formats)(
                session.handle,
                format_capacity_input,
                format_count_output,
                formats,
            )
            .result()
        }
    })
}

pub(crate) unsafe extern "system" fn xr_create_swapchain(
    session: xr::Session,
    create_info: *const xr::SwapchainCreateInfo,
    swapchain: *mut xr::Swapchain,
) -> xr::Result {
    session.run(|session| create_swapchain(session, &*create_info, &mut *swapchain))
}

fn create_swapchain(
    session: &Arc<SessionWrapper>,
    create_info: &xr::SwapchainCreateInfo,
    swapchain: &mut xr::Swapchain,
) -> Result<xr::Result> {
    let swapchain_wrapper = if let SessionGraphics::Compat {
        frontend, backend, ..
    } = &session.graphics
    {
        let format = ImageFormat::from_gl(create_info.format as u32)
            .ok_or(xr::Result::ERROR_SWAPCHAIN_FORMAT_UNSUPPORTED)?;

        let create_info2 = xr::SwapchainCreateInfo {
            ty: xr::SwapchainCreateInfo::TYPE,
            next: std::ptr::null(),
            create_flags: xr::SwapchainCreateFlags::EMPTY, //TODO
            // usage_flags: xr::SwapchainUsageFlags::TRANSFER_DST,
            usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT,
            format: format
                .to_vk()
                .ok_or(xr::Result::ERROR_SWAPCHAIN_FORMAT_UNSUPPORTED)?
                .as_raw() as i64,
            sample_count: create_info.sample_count,
            width: create_info.width,
            height: create_info.height,
            face_count: create_info.face_count,
            array_size: create_info.array_size,
            mip_count: create_info.mip_count,
        };

        unsafe {
            (session.inner.core.create_swapchain)(session.handle, &create_info2, swapchain)
                .result()?
        };

        let interop_info = graphics_interop::ImageCreateInfo {
            width: create_info.width,
            height: create_info.height,
            mip_count: create_info.mip_count,
            sample_count: create_info.sample_count,
            format,
        };

        let swapchain_backend = SwapchainBackendVulkan::load(
            *swapchain,
            &session.inner,
            backend.clone(),
            &interop_info,
        );
        let interop_handles = swapchain_backend.get_external_memory_handles();
        let swapchain_frontend = crate::graphics::opengl::frontend::SwapchainFrontendOpenGL::load(
            &interop_handles,
            frontend.clone(),
            &interop_info,
        );

        Arc::new(SwapchainWrapper {
            handle: *swapchain,
            session: Arc::downgrade(session),
            inner: session.inner.clone(),
            graphics: SwapchainGraphics::Compat {
                frontend: Box::new(swapchain_frontend),
                interop: interop_handles,
                backend: Box::new(swapchain_backend),
            },
            acquired_images: Default::default(),
            width: create_info.width,
            height: create_info.height,
        })
    } else {
        unsafe {
            (session.inner.core.create_swapchain)(session.handle, create_info, swapchain)
                .result()?;
        }
        Arc::new(SwapchainWrapper {
            handle: *swapchain,
            session: Arc::downgrade(session),
            inner: session.inner.clone(),
            graphics: SwapchainGraphics::Direct,
            acquired_images: Default::default(),
            width: create_info.width,
            height: create_info.height,
        })
    };

    xr::Swapchain::all_wrappers().insert(*swapchain, swapchain_wrapper.clone());
    session.swapchains.insert(*swapchain, swapchain_wrapper);
    info!("Swapchain created: {:?}", *swapchain);

    Ok(xr::Result::SUCCESS)
}
