#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(windows)]
pub mod windows;

use openxr::sys::platform::*;

//TODO android?

#[derive(Debug)]
pub struct Egl {
    // pub get_proc_address: PFNEGLGETPROCADDRESSPROC,
    pub display: EGLDisplay,
    pub config: EGLConfig,
    pub context: EGLContext,
}
