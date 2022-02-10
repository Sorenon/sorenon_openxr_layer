use std::ffi::CStr;

use layer_core::loader_interfaces::*;
use log::{debug, error, info};
use openxr::sys as xr;
use simplelog::*;

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "system" fn xrNegotiateLoaderApiLayerInterface(
    negotiate_info: *const XrNegotiateLoaderInfo,
    layer_name: *const i8,
    layer_request: *mut XrNegotiateApiLayerRequest,
) -> xr::Result {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Debug,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        // WriteLogger::new(LevelFilter::Info, Config::default(), File::create("").unwrap()),
    ])
    .unwrap();

    info!("Initializing layer");

    if CStr::from_ptr(layer_name).to_string_lossy() != layer_core::LAYER_NAME {
        error!(
            "Layer negotiation failed: Incorrect layer_name `{}`",
            CStr::from_ptr(layer_name).to_string_lossy()
        );
        xr::Result::ERROR_INITIALIZATION_FAILED
    } else if (*negotiate_info).min_interface_version > XR_CURRENT_LOADER_API_LAYER_VERSION
        || (*negotiate_info).max_interface_version < XR_CURRENT_LOADER_API_LAYER_VERSION
        || (*negotiate_info).min_api_version > xr::CURRENT_API_VERSION
        || (*negotiate_info).max_api_version < xr::CURRENT_API_VERSION
    {
        error!(
            "Layer negotiation failed: Incompatible negotiate info {:#?}",
            (*negotiate_info)
        );
        xr::Result::ERROR_INITIALIZATION_FAILED
    } else {
        let (get_instance_proc_addr, create_api_layer_instance) = layer_core::static_initialize();

        (*layer_request).layer_interface_version = XR_CURRENT_LOADER_API_LAYER_VERSION;
        (*layer_request).layer_api_version = xr::CURRENT_API_VERSION;
        (*layer_request).get_instance_proc_addr = Some(get_instance_proc_addr);
        (*layer_request).create_api_layer_instance = Some(create_api_layer_instance);

        debug!("Negotiation complete");

        xr::Result::SUCCESS
    }
}
