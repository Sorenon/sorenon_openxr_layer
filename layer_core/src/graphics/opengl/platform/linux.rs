use std::ffi::{c_void, CString};

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

pub struct X11 {
    pub glx: glx_sys::Glx,
    pub x_display: *mut glx_sys::types::Display,
    pub visualid: u32,
    pub glx_fb_config: glx_sys::types::GLXFBConfig,
    pub glx_drawable: glx_sys::types::GLXDrawable,
    pub glx_context: glx_sys::types::GLXContext,
}

impl X11 {
    pub fn load(
        x_display: *mut glx_sys::types::Display,
        visualid: u32,
        glx_fb_config: glx_sys::types::GLXFBConfig,
        glx_drawable: glx_sys::types::GLXDrawable,
        glx_context: glx_sys::types::GLXContext,
    ) -> Self {
        let paths = vec!["libGL.so.1", "libGL.so"];

        let lib = paths
            .iter()
            .find_map(|path| unsafe { libloading::Library::new(path).ok() })
            .unwrap();

        Self {
            glx: glx_sys::Glx::load_with(|name| unsafe {
                let addr = CString::new(name.as_bytes()).unwrap();
                lib.get(addr.as_bytes())
                    .map(|ptr| *ptr)
                    .unwrap_or(std::ptr::null())
            }),
            x_display,
            visualid,
            glx_fb_config,
            glx_drawable,
            glx_context,
        }
    }

    unsafe fn make_current(&self) {
        self.glx
            .MakeCurrent(self.x_display, self.glx_drawable, self.glx_context);
    }

    unsafe fn get_proc_address(&self, name: &str) -> *const c_void {
        let addr = CString::new(name.as_bytes()).unwrap();
        self.glx.GetProcAddress(addr.as_ptr() as _) as _
    }
}
