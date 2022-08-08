use std::ops::Deref;
use std::sync::atomic::AtomicBool;
use std::{ffi::CStr, sync::Arc};

use crate::loader_interfaces::*;
use crate::wrappers::instance::{InnerInstance, InstanceWrapper, Runtime};
use crate::wrappers::XrHandle;
use crate::ToResult;

use log::{debug, error, info, trace};

use openxr::sys as xr;
use openxr::{ExtensionSet, InstanceExtensions, Result};

pub(crate) unsafe extern "system" fn create_api_layer_instance(
    instance_info: *const xr::InstanceCreateInfo,
    layer_info: *const ApiLayerCreateInfo,
    instance: *mut xr::Instance,
) -> xr::Result {
    std::panic::catch_unwind(|| create_instance(&*instance_info, &*layer_info, &mut *instance))
        .map_or(xr::Result::ERROR_RUNTIME_FAILURE, |res| match res {
            Ok(res) => res,
            Err(res) => res,
        })
}

fn create_instance(
    instance_info: &xr::InstanceCreateInfo,
    layer_info: &ApiLayerCreateInfo,
    instance: &mut xr::Instance,
) -> Result<xr::Result> {
    let next_info = &unsafe { *layer_info.next_info };

    trace!("Create instance 222222222222222");

    if unsafe { CStr::from_ptr(std::mem::transmute(next_info.layer_name.as_ptr())) }
        .to_string_lossy()
        != crate::LAYER_NAME
    {
        error!(
            "Crate instance failed: Incorrect layer_name `{}`",
            unsafe { CStr::from_ptr(std::mem::transmute(next_info.layer_name.as_ptr())) }
                .to_string_lossy()
        );
        return Err(xr::Result::ERROR_VALIDATION_FAILURE);
    }

    debug!("Initializing OpenXR Entry");

    //Setup the OpenXR wrapper for the layer bellow us
    let entry = unsafe {
        openxr::Entry::from_get_instance_proc_addr(next_info.next_get_instance_proc_addr)?
    };

    let available_extensions = entry.enumerate_extensions()?;

    //TODO set this to true depending on env var
    let disable_opengl = true;

    //Initialize the layer bellow us
    let result = unsafe {
        let mut needs_opengl_replacement = false;

        let mut extensions = std::slice::from_raw_parts(
            instance_info.enabled_extension_names,
            instance_info.enabled_extension_count as usize,
        )
        .iter()
        .filter_map(|ext| {
            let ext_name = CStr::from_ptr(*ext).to_str().unwrap();
            if ext_name == "XR_KHR_opengl_enable" {
                if disable_opengl {
                    needs_opengl_replacement = true;
                } else if !available_extensions.khr_opengl_enable {
                    needs_opengl_replacement = true;
                    return None;
                }
            }
            Some(*ext)
        })
        .collect::<Vec<_>>();

        if needs_opengl_replacement {
            extensions.push("XR_KHR_vulkan_enable2\0".as_ptr() as *const i8);
        }

        let mut instance_info2 = *instance_info;
        instance_info2.enabled_extension_names = extensions.as_ptr();
        instance_info2.enabled_extension_count = extensions.len() as u32;

        let mut layer_info2 = *layer_info;
        layer_info2.next_info = (*layer_info2.next_info).next;

        (next_info.next_create_api_layer_instance)(&instance_info2, &layer_info2, instance).result()
    }?;

    let mut supported_extensions = ExtensionSet::default();
    supported_extensions.khr_vulkan_enable2 = true;

    let inner = unsafe {
        InnerInstance {
            poison: AtomicBool::new(false),
            core: openxr::raw::Instance::load(&entry, *instance)?,
            exts: InstanceExtensions::load(&entry, *instance, &supported_extensions)?,
        }
    };

    let runtime_name = unsafe {
        let mut instance_properties = xr::InstanceProperties::out(std::ptr::null_mut());
        (inner.core.get_instance_properties)(*instance, instance_properties.as_mut_ptr())
            .result()?;
        let instance_properties = instance_properties.assume_init();

        CStr::from_ptr(std::mem::transmute(
            instance_properties.runtime_name.as_ptr(),
        ))
        .to_string_lossy()
    };

    let runtime = match runtime_name.deref() {
        "SteamVR/OpenXR" => Runtime::SteamVR,
        "Oculus" => Runtime::Oculus,
        "Windows Mixed Reality Runtime" => Runtime::WMR,
        "Monado(XRT) by Collabora et al" => Runtime::Monado,
        _ => Runtime::Other(runtime_name.to_string()),
    };

    let wrapper = InstanceWrapper {
        handle: *instance,
        inner: Arc::new(inner),
        systems: Default::default(),
        sessions: Default::default(),
        runtime,
    };

    xr::Instance::all_wrappers().insert(*instance, Arc::new(wrapper));

    info!("Instance created with name 2 2 2 2  222`{}`", unsafe {
        CStr::from_ptr(&instance_info.application_info.application_name as _).to_string_lossy()
    });

    Ok(result)
}
