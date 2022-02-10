pub mod windows;
mod linux;
use openxr::sys::platform::*;

//TODO android?

#[derive(Debug)]
pub struct EGL {
    // pub get_proc_address: PFNEGLGETPROCADDRESSPROC,
    pub display: EGLDisplay,
    pub config: EGLConfig,
    pub context: EGLContext,
}
