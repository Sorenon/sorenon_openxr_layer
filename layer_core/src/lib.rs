mod entry;
pub mod interceptors;
#[allow(dead_code)]
pub mod loader_interfaces;
mod graphics;
pub mod wrappers;

use openxr::sys as xr;

pub const LAYER_NAME: &str = "XR_APILAYER_SORENON_compat_layer";

pub fn static_initialize() -> (
    xr::pfn::GetInstanceProcAddr,
    loader_interfaces::FnCreateApiLayerInstance,
) {
    wrappers::initialize();
    // let url = format!("vscode://vadimcn.vscode-lldb/launch/config?{{'request':'attach','pid':{}}}", std::process::id());
    // std::process::Command::new("C:\\Program Files\\VSCodium\\VSCodium.exe").arg("--open-url").arg(url).output().unwrap();
    // std::thread::sleep(std::time::Duration::from_millis(2000)); // Wait for debugger to attach

    (
        interceptors::get_instance_proc_addr,
        entry::create_api_layer_instance,
    )
}

pub trait ToResult {
    fn result(self) -> Result<Self, Self>
    where
        Self: Sized + Copy,
    {
        ToResult::result2(self, self)
    }

    fn result2<T>(self, ok: T) -> Result<T, Self>
    where
        Self: Sized + Copy;
}

impl ToResult for xr::Result {
    fn result2<T>(self, ok: T) -> Result<T, Self> {
        if self.into_raw() >= 0 {
            Ok(ok)
        } else {
            Err(self)
        }
    }
}
