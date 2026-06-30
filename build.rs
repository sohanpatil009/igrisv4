use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=icons/");
    println!("cargo:rerun-if-changed=assets/");
    println!("cargo:rerun-if-changed=igrisv3.rc");
    println!("cargo:rerun-if-changed=igrisv3.exe.manifest");
    println!("cargo:rerun-if-changed=proto/");

    // Use vendored protoc so Windows builds don't require manual protoc installation
    let protoc_path = protoc_bin_vendored::protoc_bin_path()
        .expect("protoc-bin-vendored failed to locate protoc binary");
    std::env::set_var("PROTOC", protoc_path);

    tonic_build::configure()
        .build_server(false)
        .compile_protos(&["proto/riva/proto/riva_asr.proto"], &["proto/"])?;

    #[cfg(target_os = "windows")]
    {
        let rc_path = Path::new("igrisv3.rc");
        if rc_path.exists() {
            embed_resource::compile(rc_path);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(target_os = "macos")]
        {
            let icns_icon = Path::new("icons/igris_icon.icns");
            if icns_icon.exists() {
                println!("cargo:rustc-env=ICON_PATH={}", icns_icon.display());
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            let svg_icon = Path::new("icons/igris_icon.svg");
            if svg_icon.exists() {
                println!("cargo:rustc-env=ICON_PATH={}", svg_icon.display());
            }
        }
    }

    Ok(())
}