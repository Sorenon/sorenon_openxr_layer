pub mod apis;

#[derive(Debug, Clone, Copy)]
pub struct ImageCreateInfo {
    pub width: u32,
    pub height: u32,
    pub layers: u32,
    pub mip_count: u32,
    pub sample_count: u32,
    pub format: ImageFormat,
    // pub dimension: 2D
    // pub usage: T_SRC T_DST ATTACHMENT SAMPLER
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    // Normal 32 bit formats
    Rgba8Unorm,
    Rgba8UnormSrgb,

    // Packed 32 bit formats
    Rgb10a2Unorm,

    // Normal 64 bit formats
    Rgba16Float,

    // Normal 128 bit formats
    Rgba32Float,

    // Depth-stencil formats
    Depth32Float,
    Depth24PlusStencil8,

    // Non-WGPU Depth-stencil formats
    // Depth24FloatPlusStencil8Uint,
    Depth16Unorm,
}

#[cfg(target_os = "windows")] 
pub type InteropHandle = std::os::windows::raw::HANDLE;

#[cfg(target_os = "linux")] 
pub type InteropHandle = i32;