use std::ffi::c_void;

use crate::{ImageCreateInfo, ImageFormat, InteropHandle};

pub(crate) mod bindings {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

lazy_static::lazy_static! {
    static ref GL_FORMATS: bimap::BiHashMap<ImageFormat, u32> = {
        [
            (ImageFormat::Rgba8Unorm, bindings::RGBA8),
            (ImageFormat::Rgba8UnormSrgb, bindings::SRGB8_ALPHA8),

            (ImageFormat::Rgb10a2Unorm, bindings::RGB10_A2),

            (ImageFormat::Rgba16Float, bindings::RGBA16F),

            (ImageFormat::Rgba32Float, bindings::RGBA32F),

            (ImageFormat::Depth32Float, bindings::DEPTH_COMPONENT32F),
            (ImageFormat::Depth24PlusStencil8, bindings::DEPTH24_STENCIL8),

            //ImageFormat::Depth24FloatPlusStencil8Uint
            (ImageFormat::Depth16Unorm, bindings::DEPTH_COMPONENT16),
        ]
        .into_iter()
        .collect::<bimap::BiHashMap<_, _>>()
    };
}

pub type GlError = u32;
pub type GlResult<T> = Result<T, GlError>;

pub struct OpenGLInterop {
    pub gl: bindings::Gl,
}

impl OpenGLInterop {
    pub fn new<F: Fn(&str) -> *const c_void>(f: F) -> Self {
        Self {
            gl: bindings::Gl::load_with(f),
        }
    }

    pub fn import_memory(&self, handle: InteropHandle, size: u64) -> GlResult<u32> {
        let mut mem_obj = 0;

        unsafe {
            self.gl.CreateMemoryObjectsEXT(1, &mut mem_obj);
            #[cfg(target_os = "windows")]
            self.gl.ImportMemoryWin32HandleEXT(
                mem_obj,
                size,
                bindings::HANDLE_TYPE_OPAQUE_WIN32_EXT,
                handle,
            );
            #[cfg(target_os = "linux")]
            self.gl.ImportMemoryFdEXT(
                mem_obj,
                size,
                bindings::HANDLE_TYPE_OPAQUE_FD_EXT,
                handle,
            );
        }
        if mem_obj == 0 {
            Err(unsafe { self.gl.GetError() })
        } else {
            Ok(mem_obj)
        }
    }

    pub fn import_image(
        &self,
        create_info: &ImageCreateInfo,
        mem_obj: u32,
        offset: u64,
    ) -> GlResult<u32> {
        let mut texture = 0;

        unsafe {
            self.gl
                .CreateTextures(bindings::TEXTURE_2D, 1, &mut texture);
            self.gl.TextureStorageMem2DEXT(
                texture,
                create_info.mip_count as i32,
                create_info.format.to_gl().unwrap(),
                create_info.width as i32,
                create_info.height as i32,
                mem_obj,
                offset,
            );
        }
        if texture == 0 {
            Err(unsafe { self.gl.GetError() })
        } else {
            Ok(texture)
        }
    }
}

impl ImageFormat {
    pub fn to_gl(&self) -> Option<u32> {
        GL_FORMATS.get_by_left(&self).map(|i| *i)
    }

    pub fn from_gl(gl_format: u32) -> Option<Self> {
        GL_FORMATS.get_by_right(&gl_format).map(|i| *i)
    }
}
