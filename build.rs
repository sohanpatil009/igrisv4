use std::path::Path;

fn main() {
    // Tell cargo to rerun this script if icon files change
    println!("cargo:rerun-if-changed=icons/");
    println!("cargo:rerun-if-changed=assets/");
    println!("cargo:rerun-if-changed=igrisv3.rc");
    println!("cargo:rerun-if-changed=igrisv3.exe.manifest");

    // For Windows, embed the icon resource (which now includes manifest)
    #[cfg(target_os = "windows")]
    {
        let rc_path = Path::new("igrisv3.rc");
        if rc_path.exists() {
            embed_resource::compile(rc_path);
        }
    }

    // For other platforms, just ensure the icon exists
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
}