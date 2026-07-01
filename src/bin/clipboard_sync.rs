use std::path::PathBuf;

fn main() {
    println!("═══ IGRIS Clipboard Sync ═══");
    println!("");

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        let pkg_dir = PathBuf::from("./pkg");

        let mut manager = igrisv3::eco::EcoManager::new(&pkg_dir);
        if let Err(e) = manager.initialize(&pkg_dir).await {
            eprintln!("[ERROR] Failed to initialize: {}", e);
            std::process::exit(1);
        }

        manager.enable_clipboard_sync();
        manager.config_mut().enabled = true;

        let config_path = pkg_dir.join("ecosystem/ecosystem_config.json");
        manager.config_mut().save(&config_path);

        match manager.start().await {
            Ok(_) => {
                println!("[OK] Clipboard sync is running");
                println!("     HTTP server: 0.0.0.0:53327");
                println!("     TLS proxy:   0.0.0.0:53328");
                println!("     Press Ctrl+C to stop.");
            }
            Err(e) => {
                eprintln!("[ERROR] Failed to start: {}", e);
                std::process::exit(1);
            }
        }

        tokio::signal::ctrl_c().await.unwrap();
        println!("\nShutting down...");
    });
}
