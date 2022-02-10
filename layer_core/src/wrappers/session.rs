use std::sync::{Arc, Weak};

use dashmap::DashMap;
use openxr::sys as xr;

use crate::{ToResult, graphics::{vulkan, opengl::frontend::OpenGLFrontend}};

use super::{
    instance::{InnerInstance, InstanceWrapper},
    swapchain::SwapchainWrapper,
    XrHandle, XrWrapper,
};

pub struct SessionWrapper {
    pub handle: xr::Session,
    pub instance: Weak<InstanceWrapper>,
    pub inner: Arc<InnerInstance>,
    pub graphics: SessionGraphics,
    pub swapchains: DashMap<xr::Swapchain, Arc<SwapchainWrapper>>,
}

pub enum SessionGraphics {
    Headless,
    Direct,
    Compat {
        frontend: Arc<OpenGLFrontend>,
        backend: Arc<vulkan::VkBackend>,
        swapchain_formats: Vec<i64>,
    },
}

impl SessionWrapper {
    pub fn new_direct(
        instance: &Arc<InstanceWrapper>,
        create_info: &xr::SessionCreateInfo,
    ) -> openxr::Result<Self> {
        let mut handle = xr::Session::NULL;
        unsafe { (instance.inner.core.create_session)(instance.handle, create_info, &mut handle) }
            .result2(SessionWrapper {
            handle: handle,
            instance: Arc::downgrade(instance),
            inner: instance.inner.clone(),
            graphics: SessionGraphics::Direct,
            swapchains: Default::default(),
        })
    }
}

impl XrWrapper for SessionWrapper {
    fn inner_instance(&self) -> &Arc<InnerInstance> {
        &self.inner
    }
}

impl XrHandle for xr::Session {
    type Wrapper = SessionWrapper;

    fn all_wrappers<'a>() -> &'a DashMap<Self, Arc<Self::Wrapper>>
    where
        Self: Sized + std::hash::Hash,
    {
        unsafe { super::SESSION_WRAPPERS.as_ref().unwrap() }
    }
}
