use std::{
    ffi::{c_void, CString, OsStr},
    os::windows::prelude::OsStrExt,
};

use openxr::sys::platform::*;
use winapi::shared::minwindef::HMODULE;

#[derive(Debug)]
pub enum GLContext {
    Wgl(WGL),
    Egl(super::EGL),
}

impl GLContext {
    pub fn make_current(&self) {
        unsafe {
            match &self {
                GLContext::Wgl(wgl) => wgl.make_current().unwrap(),
                GLContext::Egl(_) => todo!(),
            }
        }
    }

    pub fn get_proc_address(&self, name: &str) -> *const c_void {
        unsafe {
            match &self {
                GLContext::Wgl(wgl) => wgl.get_proc_address(name),
                GLContext::Egl(_) => todo!(),
            }
        }
    }
}

#[derive(Debug)]
pub struct WGL {
    pub h_dc: HDC,
    pub h_glrc: HGLRC,
    pub gl_library: HMODULE,
}

impl WGL {
    pub unsafe fn load(h_dc: HDC, h_glrc: HGLRC) -> Self {
        use winapi::um::libloaderapi::*;

        let name = OsStr::new("opengl32.dll")
            .encode_wide()
            .chain(Some(0).into_iter())
            .collect::<Vec<_>>();

        let gl_library = LoadLibraryW(name.as_ptr());

        if gl_library.is_null() {
            panic!();
        }

        Self {
            h_dc,
            h_glrc,
            gl_library: gl_library,
        }
    }

    unsafe fn make_current(&self) -> Result<(), u32> {
        if glutin_wgl_sys::wgl::MakeCurrent(self.h_dc as _, self.h_glrc as _) != 0 {
            Ok(())
        } else {
            Err(winapi::um::errhandlingapi::GetLastError())
        }
    }

    unsafe fn get_proc_address(&self, name: &str) -> *const c_void {
        let addr = CString::new(name.as_bytes()).unwrap();
        let p = glutin_wgl_sys::wgl::GetProcAddress(addr.as_ptr()) as *const c_void;
        if !p.is_null() {
            p
        } else {
            winapi::um::libloaderapi::GetProcAddress(self.gl_library, addr.as_ptr()) as *const _
        }
    }
}
