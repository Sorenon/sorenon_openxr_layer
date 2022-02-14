use std::ffi::{c_void, CString};

use lazy_static::lazy_static;

pub enum GLContext {
    EGl,
    X11(X11),
    Xcb,
    Wayland,
}

impl GLContext {
    pub fn make_current(&self) {
        unsafe {
            match &self {
                GLContext::EGl => todo!(),
                GLContext::X11(x11) => x11.make_current(),
                GLContext::Xcb => todo!(),
                GLContext::Wayland => todo!(),
            }
        }
    }

    pub fn get_proc_address(&self, name: &str) -> *const c_void {
        unsafe {
            match &self {
                GLContext::EGl => todo!(),
                GLContext::X11(x11) => x11.get_proc_address(name),
                GLContext::Xcb => todo!(),
                GLContext::Wayland => todo!(),
            }
        }
    }
}

use glutin_glx_sys::glx as glx_sys;

struct Glx {
    inner: glx_sys::Glx,
    _lib: libloading::Library,
}

impl std::ops::Deref for Glx {
    type Target = glx_sys::Glx;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

unsafe impl Sync for Glx {}

lazy_static! {
    static ref GLX: Option<Glx> = {
        vec!["libGL.so.1", "libGL.so"]
            .iter()
            .find_map(|path| unsafe { libloading::Library::new(path).ok() })
            .map(|lib| {
                let glx = glx_sys::Glx::load_with(|name| unsafe {
                    let addr = CString::new(name.as_bytes()).unwrap();
                    lib.get(addr.as_bytes())
                        .map(|ptr| *ptr)
                        .unwrap_or(std::ptr::null())
                });
                Glx {
                    inner: glx,
                    _lib: lib,
                }
            })
    };
}

pub struct X11 {
    pub x_display: *mut glx_sys::types::Display,
    pub visualid: u32,
    pub glx_fb_config: glx_sys::types::GLXFBConfig,
    pub glx_drawable: glx_sys::types::GLXDrawable,
    pub glx_context: glx_sys::types::GLXContext,
}

impl X11 {
    unsafe fn make_current(&self) {
        GLX.as_deref()
            .unwrap()
            .MakeCurrent(self.x_display, self.glx_drawable, self.glx_context);
    }

    unsafe fn get_proc_address(&self, name: &str) -> *const c_void {
        let addr = CString::new(name.as_bytes()).unwrap();
        GLX.as_deref().unwrap().GetProcAddress(addr.as_ptr() as _) as _
    }
}
