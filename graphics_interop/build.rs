use gl_generator::{Api, Fallbacks, Profile, Registry};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    let dest = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&dest);

    println!("cargo:rerun-if-changed=build.rs");

    let mut file_output = File::create(&dest.join("gl_bindings.rs")).unwrap();
    generate_gl_bindings(&mut file_output);
    #[cfg(windows)]
    generate_wgl_bindings(dest);
}

fn generate_gl_bindings<W>(dest: &mut W)
where
    W: Write,
{
    let gl_registry = Registry::new(
        Api::Gl,
        (4, 6),
        Profile::Core,
        Fallbacks::None,
        vec![
            "GL_AMD_depth_clamp_separate",
            "GL_APPLE_vertex_array_object",
            "GL_ARB_bindless_texture",
            "GL_ARB_blend_func_extended",
            "GL_ARB_buffer_storage",
            "GL_ARB_compute_shader",
            "GL_ARB_copy_buffer",
            "GL_ARB_debug_output",
            "GL_ARB_depth_texture",
            "GL_ARB_direct_state_access",
            "GL_ARB_draw_buffers",
            "GL_ARB_ES2_compatibility",
            "GL_ARB_ES3_compatibility",
            "GL_ARB_ES3_1_compatibility",
            "GL_ARB_ES3_2_compatibility",
            "GL_ARB_framebuffer_sRGB",
            "GL_ARB_geometry_shader4",
            "GL_ARB_gl_spirv",
            "GL_ARB_gpu_shader_fp64",
            "GL_ARB_gpu_shader_int64",
            "GL_ARB_invalidate_subdata",
            "GL_ARB_multi_draw_indirect",
            "GL_ARB_occlusion_query",
            "GL_ARB_pixel_buffer_object",
            "GL_ARB_robustness",
            "GL_ARB_seamless_cube_map",
            "GL_ARB_shader_image_load_store",
            "GL_ARB_shader_objects",
            "GL_ARB_texture_buffer_object",
            "GL_ARB_texture_float",
            "GL_ARB_texture_multisample",
            "GL_ARB_texture_rg",
            "GL_ARB_texture_rgb10_a2ui",
            "GL_ARB_texture_storage",
            "GL_ARB_transform_feedback3",
            "GL_ARB_vertex_buffer_object",
            "GL_ARB_vertex_shader",
            "GL_ATI_draw_buffers",
            "GL_ATI_meminfo",
            "GL_EXT_debug_marker",
            "GL_EXT_direct_state_access",
            "GL_EXT_memory_object",
            "GL_EXT_memory_object_fd",
            "GL_EXT_framebuffer_blit",
            "GL_EXT_framebuffer_multisample",
            "GL_EXT_framebuffer_object",
            "GL_EXT_framebuffer_sRGB",
            "GL_EXT_gpu_shader4",
            "GL_EXT_packed_depth_stencil",
            "GL_EXT_provoking_vertex",
            "GL_EXT_semaphore",
            "GL_EXT_semaphore_fd",
            "GL_EXT_texture_array",
            "GL_EXT_texture_buffer_object",
            "GL_EXT_texture_compression_s3tc",
            "GL_EXT_texture_filter_anisotropic",
            "GL_EXT_texture_integer",
            "GL_EXT_texture_sRGB",
            "GL_EXT_transform_feedback",
            "GL_GREMEDY_string_marker",
            "GL_KHR_robustness",
            "GL_NVX_gpu_memory_info",
            "GL_NV_conditional_render",
            "GL_NV_vertex_attrib_integer_64bit",
            "GL_EXT_memory_object_win32",
            "GL_EXT_memory_object_fd",
        ],
    );

    (gl_registry)
        .write_bindings(gl_generator::StructGenerator, dest)
        .unwrap();
}

#[cfg(windows)]
fn generate_wgl_bindings(dest: &Path) {
    let wgl_registry = Registry::new(
        Api::Wgl,
        (1, 0),
        Profile::Core,
        Fallbacks::None,
        vec!["WGL_NV_DX_interop", "WGL_NV_DX_interop2"],
    );

    let mut dest = File::create(&dest.join("wgl_bindings.rs")).unwrap();

    (wgl_registry)
        .write_bindings(gl_generator::StructGenerator, &mut dest)
        .unwrap();
}
