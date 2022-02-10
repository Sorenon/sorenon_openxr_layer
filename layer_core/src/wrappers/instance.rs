use std::sync::{atomic::AtomicBool, Arc};

use bitflags::bitflags;
use dashmap::DashMap;
use openxr::sys as xr;

use super::{session::SessionWrapper, XrHandle, XrWrapper};

pub struct InstanceWrapper {
    pub handle: xr::Instance,
    pub inner: Arc<InnerInstance>,
    pub systems: DashMap<xr::SystemId, SystemMeta>,
    pub sessions: DashMap<xr::Session, Arc<SessionWrapper>>,
    pub runtime: Runtime,
}

pub struct InnerInstance {
    pub poison: AtomicBool,
    pub core: openxr::raw::Instance,
    pub exts: openxr::InstanceExtensions,
}

pub struct SystemMeta {
    pub form_factor: xr::FormFactor,
    pub requirements_called: GraphicsEnableFlags,
    // pub physical_device: Option<ash::vk::PhysicalDevice>,
}

pub enum Runtime {
    SteamVR,
    Oculus,
    WMR,
    Monado,
    Other(String),
}

bitflags! {
    pub struct GraphicsEnableFlags: u8 {
        const OPENGL_GL = 0b00000001;
        const VULKAN    = 0b00000010;
        const VULKAN2   = 0b00000100;
        const D3D11     = 0b00001000;
        const D3D12     = 0b00010000;
    }
}

impl XrWrapper for InstanceWrapper {
    fn inner_instance(&self) -> &Arc<InnerInstance> {
        &self.inner
    }
}

impl XrHandle for xr::Instance {
    type Wrapper = InstanceWrapper;

    fn all_wrappers<'a>() -> &'a DashMap<Self, Arc<Self::Wrapper>>
    where
        Self: Sized + std::hash::Hash,
    {
        unsafe { super::INSTANCE_WRAPPERS.as_ref().unwrap() }
    }
}
