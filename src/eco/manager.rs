use crate::eco::clipboard::{ClipboardData, ClipboardManager};
use crate::eco::config::EcosystemConfig;
use crate::eco::constants::*;
use crate::eco::crypto::EcoCrypto;
use crate::eco::device::{Capabilities, EcoDevice};
use crate::eco::discovery::DeviceDiscovery;
use crate::eco::errors::{EcoError, EcoResult};
use crate::eco::events::{EcoEvent, EventBus};
use crate::eco::notifications::NotificationManager;
use crate::eco::permissions::EcoPermissions;
use crate::eco::storage::EcoStorage;
use crate::eco::sync::SyncManager;
use crate::eco::transport::EcoTransport;
use crate::platform::ecosystem::create_platform_clipboard;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct EcoManager {
    config: EcosystemConfig,
    event_bus: Arc<EventBus>,
    transport: Arc<EcoTransport>,
    local_device: Arc<RwLock<EcoDevice>>,
    discovery: Option<Arc<DeviceDiscovery>>,
    clipboard: Option<Arc<std::sync::Mutex<ClipboardManager>>>,
    storage: Arc<std::sync::Mutex<EcoStorage>>,
    permissions: Option<Arc<EcoPermissions>>,
    crypto: Option<EcoCrypto>,
    sync: Option<Arc<SyncManager>>,
    notifications: Option<Arc<NotificationManager>>,
    eco_port: u16,
    initialized: bool,
    running: bool,
}

impl EcoManager {
    pub fn new(pkg_dir: &PathBuf) -> Self {
        let config_path = pkg_dir.join(ECO_STORAGE_DIR).join(ECO_CONFIG_FILE);
        let config = EcosystemConfig::from_file(&config_path).unwrap_or_default();

        let event_bus = Arc::new(EventBus::new());
        let transport = Arc::new(EcoTransport::new());

        let device_name = config.device_name.clone();
        let local_device = Arc::new(RwLock::new(EcoDevice::new(device_name)));

        let storage = Arc::new(std::sync::Mutex::new(EcoStorage::new(pkg_dir)));

        Self {
            config,
            event_bus,
            transport,
            local_device,
            discovery: None,
            clipboard: None,
            storage,
            permissions: None,
            crypto: None,
            sync: None,
            notifications: None,
            eco_port: DEFAULT_ECO_PORT,
            initialized: false,
            running: false,
        }
    }

    pub async fn initialize(&mut self, pkg_dir: &PathBuf) -> EcoResult<()> {
        if self.initialized {
            return Ok(());
        }

        let storage = EcoStorage::new(pkg_dir);
        storage.init_dirs()?;
        self.storage = Arc::new(std::sync::Mutex::new(storage));

        let crypto = EcoCrypto::new(pkg_dir)?;
        let public_key_pem = crypto.public_key_pem();

        {
            let mut device = self.local_device.write().await;
            device.public_key = Some(public_key_pem);
            device.capabilities = Capabilities {
                clipboard_sync: self.config.clipboard_sync,
                ..Default::default()
            };
        }

        let permissions = EcoPermissions::new(
            EcoStorage::new(pkg_dir)
        );
        self.permissions = Some(Arc::new(permissions));
        self.crypto = Some(crypto);

        let clipboard_platform = create_platform_clipboard();
        let clipboard = ClipboardManager::new(
            clipboard_platform,
            self.event_bus.clone(),
            self.storage.clone(),
        );
        self.clipboard = Some(Arc::new(std::sync::Mutex::new(clipboard)));

        self.initialized = true;
        self.running = false;

        self.event_bus.emit(EcoEvent::Synced("Ecosystem initialized".to_string()));

        Ok(())
    }

    pub async fn start(&mut self) -> EcoResult<()> {
        if !self.initialized {
            return Err(EcoError::NotInitialized);
        }
        if self.running {
            return Ok(());
        }

        if !self.config.enabled {
            return Ok(());
        }

        // ---- Bind ecosystem HTTP server port ----
        let mut port = self.config.port;
        let mut bound = false;
        for offset in 0..PORT_RANGE {
            let try_port = port + offset;
            let addr: SocketAddr = match format!("0.0.0.0:{}", try_port).parse() {
                Ok(a) => a,
                Err(_) => continue,
            };
            if tokio::net::TcpListener::bind(addr).await.is_ok() {
                port = try_port;
                bound = true;
                break;
            }
        }
        if !bound {
            return Err(EcoError::Transport("Could not bind to any port".to_string()));
        }
        self.eco_port = port;

        // ---- Start TLS proxy (eco TLS port -> eco HTTP port) ----
        let tls_cfg = tokio::task::spawn_blocking(|| crate::fastswap::tls::get_or_create_tls_config())
            .await
            .map_err(|e| EcoError::Transport(format!("TLS init panicked: {}", e)))?
            .map_err(|e| EcoError::Transport(format!("TLS init error: {}", e)))?;

        let eco_tls_port = if port == DEFAULT_ECO_PORT { ECO_TLS_PORT } else { port + 1 };
        let _ = crate::fastswap::network::start_tls_proxy(eco_tls_port, port, tls_cfg.server_config)
            .await
            .map_err(|e| EcoError::Transport(format!("TLS proxy error: {}", e)))?;

        // ---- Start ecosystem HTTP server (clipboard endpoint) ----
        let http_addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
        let transport = self.transport.clone();
        let local_device = self.local_device.clone();

        let discovery = Arc::new(DeviceDiscovery::new(
            self.event_bus.clone(),
        ));
        discovery.start_server(&http_addr).await;
        discovery.start_discovery().await;
        discovery.run_cleanup().await;

        let known_devices = discovery.get_known_devices();

        // ---- Sync manager (sends clipboard changes to peers via HTTPS) ----
        let sync = Arc::new(SyncManager::new(
            self.event_bus.clone(),
            transport.clone(),
            known_devices.clone(),
            self.local_device.clone(),
        ));
        let _ = sync.start().await;
        self.sync = Some(sync);

        // ---- Notification manager (broadcasts & receives notifications) ----
        let notification_manager = Arc::new(NotificationManager::new(
            self.event_bus.clone(),
            transport,
            known_devices,
            self.storage.clone(),
        ));
        crate::eco::notifications::init_manager(notification_manager.clone());
        notification_manager.start();
        self.notifications = Some(notification_manager);

        // ---- Apply received clipboard data to local clipboard ----
        {
            let event_bus = self.event_bus.clone();
            let manager_ptr = self.clipboard.clone().unwrap();
            event_bus.subscribe(Arc::new(move |event| {
                if let EcoEvent::ClipboardReceived(data, _from) = event {
                    println!("[ECO] Applying received clipboard (hash={})", &data.content_hash[..16]);
                    let manager = manager_ptr.clone();
                    tokio::spawn(async move {
                        if let Ok(mut guard) = manager.lock() {
                            let _ = guard.apply_clipboard(&data);
                        }
                    });
                }
            }));
        }

        self.discovery = Some(discovery);

        println!("[ECO] HTTP server on port {}, TLS proxy on port {}", port, eco_tls_port);
        println!("[ECO] Discovery via HTTPS subnet scan on port {}", eco_tls_port);

        if self.config.clipboard_sync {
            println!("[ECO] Clipboard monitoring ENABLED (polling every 1s)");
            if let Some(clipboard) = &self.clipboard {
                let cb = clipboard.clone();
                tokio::spawn(async move {
                    ClipboardManager::start_monitoring(cb).await;
                });
            }
        }

        self.running = true;

        self.event_bus.emit(EcoEvent::Synced(
            format!("Ecosystem started — HTTP:{} TLS:{}", port, eco_tls_port)
        ));

        Ok(())
    }

    pub async fn shutdown(&mut self) {
        if !self.running {
            return;
        }
        self.running = false;
        self.event_bus.emit(EcoEvent::Synced("Ecosystem shut down".to_string()));
    }

    pub fn config(&self) -> &EcosystemConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut EcosystemConfig {
        &mut self.config
    }

    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    pub fn local_device(&self) -> &Arc<RwLock<EcoDevice>> {
        &self.local_device
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn enable_clipboard_sync(&mut self) {
        self.config.clipboard_sync = true;
    }

    pub fn disable_clipboard_sync(&mut self) {
        self.config.clipboard_sync = false;
    }

    pub async fn apply_received_clipboard(&self, data: ClipboardData) -> EcoResult<()> {
        if let Some(clipboard) = &self.clipboard {
            let mut guard = clipboard.lock().map_err(|_| {
                EcoError::Clipboard("Lock failed".to_string())
            })?;
            guard.apply_clipboard(&data)
        } else {
            Err(EcoError::NotInitialized)
        }
    }
}

lazy_static::lazy_static! {
    pub static ref ECO_MANAGER: std::sync::Mutex<Option<EcoManager>> = std::sync::Mutex::new(None);
}

pub fn get_eco_manager() -> Option<std::sync::MutexGuard<'static, Option<EcoManager>>> {
    ECO_MANAGER.lock().ok()
}

pub fn init_eco_manager(pkg_dir: &PathBuf) -> EcoResult<()> {
    let mut manager = EcoManager::new(pkg_dir);
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| EcoError::Transport(e.to_string()))?;
    runtime.block_on(manager.initialize(pkg_dir))?;

    let mut guard = ECO_MANAGER.lock()
        .map_err(|_| EcoError::Transport("Lock failed".to_string()))?;
    *guard = Some(manager);
    Ok(())
}

pub fn start_eco_manager() -> EcoResult<()> {
    let mut guard = ECO_MANAGER.lock()
        .map_err(|_| EcoError::Transport("Lock failed".to_string()))?;
    if let Some(ref mut manager) = *guard {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| EcoError::Transport(e.to_string()))?;
        runtime.block_on(manager.start())?;
        Ok(())
    } else {
        Err(EcoError::NotInitialized)
    }
}

pub fn shutdown_eco_manager() {
    if let Ok(mut guard) = ECO_MANAGER.lock() {
        if let Some(ref mut manager) = *guard {
            let runtime = tokio::runtime::Runtime::new().ok();
            if let Some(rt) = runtime {
                rt.block_on(manager.shutdown());
            }
        }
    }
}