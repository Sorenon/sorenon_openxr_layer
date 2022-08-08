use openxr::sys::*;

pub const XR_CURRENT_LOADER_API_LAYER_VERSION: u32 = 1;
pub const XR_CURRENT_LOADER_RUNTIME_VERSION: u32 = 1;

pub type FnCreateApiLayerInstance = unsafe extern "system" fn(
    info: *const InstanceCreateInfo,
    api_layer_info: *const ApiLayerCreateInfo,
    instance: *mut Instance,
) -> Result;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct XrNegotiateLoaderInfo {
    pub ty: StructureType,
    pub struct_version: u32,
    pub struct_size: usize,
    pub min_interface_version: u32,
    pub max_interface_version: u32,
    pub min_api_version: Version,
    pub max_api_version: Version,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XrNegotiateApiLayerRequest {
    pub ty: StructureType,
    pub struct_version: u32,
    pub struct_size: usize,
    pub layer_interface_version: u32,
    pub layer_api_version: Version,
    pub get_instance_proc_addr: Option<pfn::GetInstanceProcAddr>,
    pub create_api_layer_instance: Option<FnCreateApiLayerInstance>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XrNegotiateRuntimeRequest {
    pub ty: StructureType,
    pub struct_version: u32,
    pub struct_size: usize,
    pub runtime_interface_version: u32,
    pub runtime_api_version: Version,
    pub get_instance_proc_addr: Option<pfn::GetInstanceProcAddr>,
}

pub type FnNegotiateLoaderApiLayerInterface = unsafe extern "system" fn(
    loader_info: *const XrNegotiateLoaderInfo,
    api_layer_name: *const i8,
    api_layer_request: *mut XrNegotiateApiLayerRequest,
) -> Result;

pub type FnNegotiateLoaderRuntimeInterface = unsafe extern "system" fn(
    loader_info: *const XrNegotiateLoaderInfo,
    runtime_request: *mut XrNegotiateRuntimeRequest,
) -> Result;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XrApiLayerNextInfo {
    pub ty: StructureType,
    pub struct_version: u32,
    pub struct_size: usize,
    pub layer_name: [i8; MAX_API_LAYER_NAME_SIZE],
    pub next_get_instance_proc_addr: pfn::GetInstanceProcAddr,
    pub next_create_api_layer_instance: FnCreateApiLayerInstance,
    pub next: *mut XrApiLayerNextInfo,
}

pub const XR_API_LAYER_MAX_SETTINGS_PATH_SIZE: usize = 512usize;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ApiLayerCreateInfo {
    pub ty: StructureType,
    pub struct_version: u32,
    pub struct_size: usize,
    pub loader_instance: *const (),
    pub settings_file_location: [i8; XR_API_LAYER_MAX_SETTINGS_PATH_SIZE],
    pub next_info: *mut XrApiLayerNextInfo,
}
