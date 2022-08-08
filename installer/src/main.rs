use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(windows)]
fn main() {
    todo!()
}

#[cfg(target_os = "linux")]
fn main() {
    let mut args = std::env::args();
    args.next().unwrap();

    if let Some(arg) = args.next() {
        match &arg[..] {
            "install" => install(),
            "uninstall" => uninstall(),
            _ => panic!("Unexpected argument `{}`", arg),
        }
    } else {
        install()
    }
}

fn uninstall() {
    let path = manifest_path().unwrap();
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
        println!("Successfully deleted `{}`", path.display());
    } else {
        eprintln!("Layer not installed");
    }
}

fn install() {
    let layer_path = layer_path();
    if !Path::new(&layer_path).exists() {
        panic!("Could not find layer at `{}`\nTry building crate in release mode (cargo run --release)", layer_path)
    }
    let path = manifest_path().unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = File::create(&path).unwrap();
    file.write_all(json_contents(&layer_path).as_bytes())
        .unwrap();
    println!("Successfully installed layer in `{}`", path.display());
}

fn layer_path() -> String {
    let workspace_path = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    workspace_path
        .join(Path::new("target/release/liblayer_entry.so"))
        .to_str()
        .unwrap()
        .to_owned()
}

fn manifest_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| {
        home.join(Path::new(
            ".local/share/openxr/1/api_layers/implicit.d/sorenon_layer.json",
        ))
    })
}

fn json_contents(shared_lib_path: &str) -> String {
    r#"{
    "file_format_version" : "1.0.0",
    "api_layer": {
        "name": "XR_APILAYER_SORENON_compat_layer",
        "library_path": ""#
        .to_owned()
        + shared_lib_path
        + r#"",
        "api_version" : "1.0",
        "implementation_version" : "1",
        "description" : "Provides OpenGL over Vulkan",
        "instance_extensions": [
            {
                "name": "XR_KHR_opengl_enable",
                "extension_version": "10"
            }
        ],
        "disable_environment": "DISABLE_SORENON_OPENXR_LAYER"
    }
}"#
}
