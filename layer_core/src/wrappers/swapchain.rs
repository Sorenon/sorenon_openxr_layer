use std::{
    cell::RefCell,
    collections::VecDeque,
    sync::{Arc, Mutex, Weak},
};

use dashmap::DashMap;
use log::error;
use openxr::sys as xr;

use super::{instance::InnerInstance, session::SessionWrapper, XrHandle, XrWrapper};

pub struct SwapchainWrapper {
    pub handle: xr::Swapchain,
    pub session: Weak<SessionWrapper>,
    pub inner: Arc<InnerInstance>,
    pub width: u32,
    pub height: u32,
    pub layers: u32,
    pub graphics: SwapchainGraphics,
    pub acquired_images: Mutex<RefCell<VecDeque<u32>>>,
}

pub enum SwapchainGraphics {
    Direct,
    Compat {
        frontend: Box<dyn SwapchainFrontend>,
        interop: Vec<(graphics_interop::InteropHandle, u64)>,
        backend: Box<dyn SwapchainBackend>,
    },
}

pub trait SwapchainFrontend {
    unsafe fn enumerate_images(
        &self,
        capacity: u32,
        count_output: *mut u32,
        out: *mut xr::SwapchainImageBaseHeader,
    ) -> openxr::Result<xr::Result>;

    fn release_image(&self, index: u32);

    fn destroy(&self);
}

pub trait SwapchainBackend {
    fn get_external_memory_handles(&self) -> Vec<(graphics_interop::InteropHandle, u64)>;

    fn release_image(&self, index: usize);

    fn destroy(&self);
}

impl Drop for SwapchainWrapper {
    fn drop(&mut self) {
        if let SwapchainGraphics::Compat {
            frontend,
            interop,
            backend,
        } = &self.graphics
        {
            frontend.destroy();
            for &(handle, _) in interop {
                #[cfg(target_os = "windows")]
                unsafe {
                    winapi::um::handleapi::CloseHandle(handle);
                }
                #[cfg(target_os = "linux")]
                unsafe {
                    if libc::close(handle) == -1 {
                        error!(
                            "Failed to close swapchain fd `{}` with error `{:X}`",
                            handle,
                            *libc::__errno_location()
                        )
                    }
                }
            }
            backend.destroy();
        }
    }
}

impl XrWrapper for SwapchainWrapper {
    fn inner_instance(&self) -> &Arc<InnerInstance> {
        &self.inner
    }
}

impl XrHandle for xr::Swapchain {
    type Wrapper = SwapchainWrapper;

    fn all_wrappers<'a>() -> &'a DashMap<Self, Arc<Self::Wrapper>>
    where
        Self: Sized + std::hash::Hash,
    {
        unsafe { super::SWAPCHAIN_WRAPPERS.as_ref().unwrap() }
    }
}
