use std::sync::Arc;

use graphics_interop::apis::opengl::OpenGLInterop;
use openxr::sys as xr;

use crate::wrappers::swapchain::SwapchainFrontend;

use super::GLContext;

pub struct OpenGLFrontend {
    pub interop: OpenGLInterop,
    pub context: GLContext,
}

impl OpenGLFrontend {
    pub fn load(context: GLContext) -> Self {
        Self {
            interop: graphics_interop::apis::opengl::OpenGLInterop::new(|name| {
                context.get_proc_address(name)
            }),
            context,
        }
    }
}

pub struct SwapchainFrontendOpenGL {
    opengl: Arc<OpenGLFrontend>,
    memory_objects: Vec<u32>,
    images: Vec<u32>,
}

impl SwapchainFrontendOpenGL {
    pub fn load(
        handles: &[(graphics_interop::InteropHandle, u64)],
        opengl: Arc<OpenGLFrontend>,
        image_info: &graphics_interop::ImageCreateInfo,
    ) -> Self {
        let mut memory_objects = Vec::with_capacity(handles.len());
        let mut images = Vec::with_capacity(handles.len());

        for &(handle, size) in handles {
            memory_objects.push(opengl.interop.import_memory(handle, size).unwrap());
            images.push(
                opengl
                    .interop
                    .import_image(image_info, *memory_objects.last().unwrap(), 0)
                    .unwrap(),
            );
        }

        Self {
            opengl,
            memory_objects,
            images,
        }
    }
}

impl SwapchainFrontend for SwapchainFrontendOpenGL {
    unsafe fn enumerate_images(
        &self,
        capacity: u32,
        count_output: *mut u32,
        out: *mut openxr::sys::SwapchainImageBaseHeader,
    ) -> openxr::Result<xr::Result> {
        if capacity != 0 {
            if (capacity as usize) < self.images.len() {
                return Err(xr::Result::ERROR_SIZE_INSUFFICIENT);
            }
            if out.is_null() {
                return Err(xr::Result::ERROR_VALIDATION_FAILURE);
            }
            let slice: &mut [xr::SwapchainImageOpenGLKHR] =
                std::slice::from_raw_parts_mut(std::mem::transmute(out), self.images.len());
            for (i, image_out) in slice.iter_mut().enumerate() {
                if image_out.ty != xr::SwapchainImageOpenGLKHR::TYPE {
                    return Err(xr::Result::ERROR_VALIDATION_FAILURE);
                }
                image_out.image = self.images[i];
            }
        }
        if count_output.is_null() {
            return Err(xr::Result::ERROR_VALIDATION_FAILURE);
        }
        *count_output = self.images.len() as u32;
        Ok(xr::Result::SUCCESS)
    }

    fn release_image(&self, _: u32) {
        self.opengl.context.make_current();
        unsafe {
            //We need to wait for all OpenGL calls to finish execution before copying the image
            self.opengl.interop.gl.Finish();

            //TODO use fences on android
        }
    }

    fn destroy(&self) {
        unsafe {
            self.opengl
                .interop
                .gl
                .DeleteTextures(self.images.len() as i32, self.images.as_ptr());
            self.opengl.interop.gl.DeleteMemoryObjectsEXT(
                self.memory_objects.len() as i32,
                self.memory_objects.as_ptr(),
            )
        }
    }
}
