pub mod instance;
mod session;
mod swapchain;

use std::{ffi::CStr, os::raw::c_char};

use log::{error, trace, warn};
use openxr::sys::{self as xr, pfn};
use openxr::Result;

use crate::wrappers::instance::InstanceWrapper;
use crate::wrappers::XrHandle;
use crate::ToResult;

pub(crate) unsafe extern "system" fn get_instance_proc_addr(
    instance: xr::Instance,
    name: *const c_char,
    function: *mut Option<pfn::VoidFunction>,
) -> xr::Result {
    instance.run(|instance| instance_proc_addr(instance, name, &mut *function))
}

const INTERCEPTORS: [unsafe fn(&str) -> Option<pfn::VoidFunction>; 3] = [
    instance::get_instance_interceptors,
    session::get_session_interceptors,
    swapchain::get_swapchain_interceptors,
];

fn instance_proc_addr(
    instance: &InstanceWrapper,
    name: *const c_char,
    function: &mut Option<pfn::VoidFunction>,
) -> Result<xr::Result> {
    let name_str = unsafe { CStr::from_ptr(name) }.to_str().map_err(|err| {
        warn!(
            "get_instance_proc_addr passed bad name ({}): `{}`",
            unsafe { CStr::from_ptr(name) }.to_string_lossy(),
            err,
        );
        //We can't parse the function name so just let the runtime deal with it
        unsafe { (instance.inner.core.get_instance_proc_addr)(instance.handle, name, function) }
    })?;

    trace!("get_instance_proc_addr({})", name_str);

    (*function) = unsafe { INTERCEPTORS.iter().find_map(|f| f(name_str)) };

    if function.is_some() {
        Ok(xr::Result::SUCCESS)
    } else {
        unsafe { (instance.inner.core.get_instance_proc_addr)(instance.handle, name, function) }
            .result()
    }
}

unsafe fn enumerate<T: Copy>(
    capacity: u32,
    count_output: *mut u32,
    out: *mut T,
    data: &[T],
) -> Result<xr::Result> {
    if capacity != 0 {
        if (capacity as usize) < data.len() {
            return Err(xr::Result::ERROR_SIZE_INSUFFICIENT);
        }
        if out.is_null() {
            return Err(xr::Result::ERROR_VALIDATION_FAILURE);
        }
        let slice = std::slice::from_raw_parts_mut(out, data.len());
        slice.copy_from_slice(data);
    }
    if count_output.is_null() {
        return Err(xr::Result::ERROR_VALIDATION_FAILURE);
    }
    *count_output = data.len() as u32;
    Ok(xr::Result::SUCCESS)
}

type Func<H, T> = unsafe extern "system" fn(
    handle: H,
    format_capacity_input: u32,
    format_count_output: *mut u32,
    out: *mut T,
) -> xr::Result;

pub unsafe fn call_enumerate<H: Copy, T: Copy>(
    handle: H,
    f: Func<H, T>,
    default: T,
) -> Result<Vec<T>> {
    let mut count = 0;

    f(handle, 0, &mut count, std::ptr::null_mut())
        .result()
        .map_err(|err| {
            error!("1{}", err);
            xr::Result::ERROR_RUNTIME_FAILURE
        })?;

    let mut vec = vec![default; count as usize];

    f(handle, count, &mut count, vec.as_mut_ptr())
        .result()
        .map_err(|err| {
            error!("2{} {}", count, err);
            xr::Result::ERROR_RUNTIME_FAILURE
        })?;
    Ok(vec)
}
