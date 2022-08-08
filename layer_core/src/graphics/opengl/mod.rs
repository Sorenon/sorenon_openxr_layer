pub mod frontend;

pub mod platform;

#[cfg(target_os = "linux")]
pub type GLContext = platform::linux::GLContext;

#[cfg(windows)]
pub type GLContext = platform::windows::GLContext;
