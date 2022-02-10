pub mod frontend;
pub mod platform;

#[cfg(windows)]
pub type GLContext = platform::windows::GLContext;