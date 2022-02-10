use openxr::{
    sys::{self as xr, pfn},
    Result,
};

use crate::{
    wrappers::{
        swapchain::{SwapchainGraphics, SwapchainWrapper},
        XrHandle,
    },
    ToResult,
};

pub(super) unsafe fn get_swapchain_interceptors(name: &str) -> Option<pfn::VoidFunction> {
    use std::mem::transmute;
    use xr::pfn::*;
    Some(match name {
        "xrEnumerateSwapchainImages" => {
            transmute(xr_enumerate_swapchain_images as EnumerateSwapchainImages)
        }
        "xrAcquireSwapchainImage" => transmute(xr_acquire_swapchain_image as AcquireSwapchainImage),
        "xrReleaseSwapchainImage" => transmute(xr_release_swapchain_image as ReleaseSwapchainImage),
        _ => return None,
    })
}

unsafe extern "system" fn xr_enumerate_swapchain_images(
    swapchain: xr::Swapchain,
    image_capacity_input: u32,
    image_count_output: *mut u32,
    images: *mut xr::SwapchainImageBaseHeader,
) -> xr::Result {
    swapchain.run(|swapchain| {
        if let SwapchainGraphics::Compat { frontend, .. } = &swapchain.graphics {
            frontend.enumerate_images(image_capacity_input, image_count_output, images)
        } else {
            (swapchain.inner.core.enumerate_swapchain_images)(
                swapchain.handle,
                image_capacity_input,
                image_count_output,
                images,
            )
            .result()
        }
    })
}

unsafe extern "system" fn xr_acquire_swapchain_image(
    swapchain: xr::Swapchain,
    acquire_info: *const xr::SwapchainImageAcquireInfo,
    index: *mut u32,
) -> xr::Result {
    swapchain.run(|swapchain| acquire_swapchain_image(swapchain, &*acquire_info, &mut *index))
}

unsafe extern "system" fn xr_release_swapchain_image(
    swapchain: xr::Swapchain,
    release_info: *const xr::SwapchainImageReleaseInfo,
) -> xr::Result {
    swapchain.run(|swapchain| release_swapchain_image(swapchain, &*release_info))
}

fn acquire_swapchain_image(
    swapchain: &SwapchainWrapper,
    acquire_info: &xr::SwapchainImageAcquireInfo,
    index: &mut u32,
) -> Result<xr::Result> {
    let success = unsafe {
        (swapchain.inner.core.acquire_swapchain_image)(swapchain.handle, acquire_info, index)
    }
    .result()?;

    swapchain
        .acquired_images
        .lock()
        .unwrap()
        .get_mut()
        .push_back(*index);

    Ok(success)
}

fn release_swapchain_image(
    swapchain: &SwapchainWrapper,
    release_info: &xr::SwapchainImageReleaseInfo,
) -> Result<xr::Result> {
    let mut lock = swapchain.acquired_images.lock().unwrap();
    let queue = lock.get_mut();

    if let SwapchainGraphics::Compat {
        frontend, backend, ..
    } = &swapchain.graphics
    {
        let index = *queue.front().unwrap();
        //TODO better sub resource memory format transitions
        frontend.release_image(index);
        backend.release_image(index as usize);
        // let runtime_image = backend.runtime_images[index as usize];
        // let image = backend.images[index as usize];
        // vk_base.record_submit_commandbuffer(
        //     vk_base.command_buffer,
        //     vk::Fence::null(),
        //     &[],
        //     &[],
        //     &[],
        //     |device, cmd| {
        //         device.cmd_copy_image(
        //             cmd,
        //             image,
        //             vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        //             runtime_image,
        //             vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        //             &[vk::ImageCopy {
        //                 src_subresource: vk::ImageSubresourceLayers {
        //                     aspect_mask: vk::ImageAspectFlags::COLOR,
        //                     mip_level: 0,
        //                     base_array_layer: 0,
        //                     layer_count: 1,
        //                 },
        //                 src_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
        //                 dst_subresource: vk::ImageSubresourceLayers {
        //                     aspect_mask: vk::ImageAspectFlags::COLOR,
        //                     mip_level: 0,
        //                     base_array_layer: 0,
        //                     layer_count: 1,
        //                 },
        //                 dst_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
        //                 extent: vk::Extent3D {
        //                     width: swapchain.width,
        //                     height: swapchain.height,
        //                     depth: 1,
        //                 },
        //             }],
        //         )
        //     },
        // );
        // vk_base.device.device_wait_idle().unwrap();
    }
    let success =
        unsafe { (swapchain.inner.core.release_swapchain_image)(swapchain.handle, release_info) }
            .result()?;

    queue.pop_front().unwrap();

    Ok(success)
}
