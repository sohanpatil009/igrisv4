// FastSwap integration module for IGRIS
// Based on localshare-desktop implementation

pub mod models;
pub mod network;
pub mod tls;

pub use models::*;
pub use network::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use anyhow::Result;

// Global progress tracker for UI access
static GLOBAL_PROGRESS_TRACKER: once_cell::sync::Lazy<ProgressTracker> =
    once_cell::sync::Lazy::new(|| models::progress::create_progress_tracker());

// Global pending transfers (for receiver approval)
#[derive(Clone, Debug)]
pub struct PendingTransfer {
    pub session_id: String,
    pub sender_name: String,
    pub sender_device: String,
    pub file_count: usize,
    pub total_size: u64,
    pub files: Vec<String>,
}

static PENDING_TRANSFERS: once_cell::sync::Lazy<Arc<RwLock<Vec<PendingTransfer>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(Vec::new())));

static APPROVED_SESSIONS: once_cell::sync::Lazy<Arc<RwLock<Vec<String>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(Vec::new())));

/// Notifiers for pending approval — server awaits these instead of polling
static APPROVAL_NOTIFIERS: once_cell::sync::Lazy<
    Arc<RwLock<std::collections::HashMap<String, Arc<tokio::sync::Notify>>>>,
> = once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(std::collections::HashMap::new())));

/// Register a watch for approval on a session. The returned `Notify` will fire
/// when `approve_transfer` or `deny_transfer` is called.
pub async fn register_approval_watch(session_id: &str) -> Arc<tokio::sync::Notify> {
    let mut map = APPROVAL_NOTIFIERS.write().await;
    let n = Arc::new(tokio::sync::Notify::new());
    map.insert(session_id.to_string(), n.clone());
    n
}

// ---- Cancellation registry ----
// Allows the UI to cancel an in-progress transfer by session ID.
// The sender's upload tasks watch this channel between chunks.

static CANCELLATION_REGISTRY: once_cell::sync::Lazy<
    Arc<RwLock<HashMap<String, tokio::sync::watch::Sender<bool>>>>,
> = once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Register a cancellation sender for a session. The sender should drop
/// the returned `Receiver` when the transfer completes so the entry is
/// cleaned up.
pub async fn register_cancellation(session_id: &str, tx: tokio::sync::watch::Sender<bool>) {
    CANCELLATION_REGISTRY
        .write()
        .await
        .insert(session_id.to_string(), tx);
}

/// Unregister a cancellation sender (called when transfer completes naturally).
pub async fn unregister_cancellation(session_id: &str) {
    CANCELLATION_REGISTRY.write().await.remove(session_id);
}

/// Notify all watchers that a transfer should be cancelled.
pub async fn notify_cancellation(session_id: &str) {
    if let Some(tx) = CANCELLATION_REGISTRY.write().await.remove(session_id) {
        let _ = tx.send(true);
    }
}

/// FastSwap manager for IGRIS integration
pub struct FastSwapManager {
    discovery: Arc<RwLock<DiscoveryService>>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    port: u16,
}

impl FastSwapManager {
    pub fn new(port: u16) -> Self {
        let discovery = Arc::new(RwLock::new(DiscoveryService::new()));
        
        Self {
            discovery,
            server_handle: None,
            port,
        }
    }
    
    pub async fn start(&mut self, local_device: Device) -> Result<()> {
        let port = self.port;

        // Start HTTP server (blocks until bound, then runs in background)
        let http_port = network::start_server(port, local_device.clone())
            .await
            .map_err(|e| anyhow::anyhow!("HTTP server error: {}", e))?;

        tracing::info!("[FastSwap] Generating/loading TLS certificate...");
        let tls_cfg = tokio::task::spawn_blocking(|| tls::get_or_create_tls_config())
            .await
            .map_err(|e| anyhow::anyhow!("TLS init task panicked: {}", e))?
            .map_err(|e| anyhow::anyhow!("TLS init error: {}", e))?;

        let tls_port = if http_port == 53317 { 53318 } else { http_port + 1 };

        let proxy_handle = tokio::spawn(async move {
            let _ = network::start_tls_proxy(tls_port, http_port, tls_cfg.server_config).await;
        });

        self.server_handle = Some(proxy_handle);
        self.port = http_port;

        tracing::info!("[FastSwap] HTTP:{}, TLS:{}", http_port, tls_port);
        Ok(())
    }
    
    pub async fn stop(&mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
        tracing::info!("[FastSwap] Stopped");
    }
    
    pub async fn get_devices(&self) -> Vec<Device> {
        self.discovery.read().await.get_devices().await
    }
    
    pub fn get_discovery_service(&self) -> Arc<RwLock<DiscoveryService>> {
        Arc::clone(&self.discovery)
    }
}

// ---- On-demand start infrastructure ----
// Manager + device are stored here unstarted; server boots when the
// FastSwap panel is first opened or a voice command triggers it.

pub static FASTSWAP_MANAGER: once_cell::sync::Lazy<Arc<Mutex<Option<FastSwapManager>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));
pub static FASTSWAP_DEVICE: once_cell::sync::Lazy<Arc<Mutex<Option<Device>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// Returns true if the FastSwap server has already been started.
pub fn is_fastswap_running() -> bool {
    if let Ok(guard) = FASTSWAP_MANAGER.lock() {
        return guard
            .as_ref()
            .map(|m| m.server_handle.is_some())
            .unwrap_or(false);
    }
    false
}

/// Start the FastSwap server if it has not already been started.
/// Safe to call multiple times — subsequent calls are no-ops.
pub async fn start_on_demand() {
    if is_fastswap_running() {
        return;
    }

    let local_device = {
        let dev_guard = FASTSWAP_DEVICE.lock().unwrap();
        dev_guard.clone()
    };

    let local_device = match local_device {
        Some(d) => d,
        None => {
            // Compute on first access (fallback)
            let local_ip = local_ip_address::local_ip()
                .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 100)))
                .to_string();
            let device = Device::new_local(
                format!("IGRIS-{}", whoami::username()),
                53317,
                local_ip.clone(),
            );
            let mut dev_guard = FASTSWAP_DEVICE.lock().unwrap();
            *dev_guard = Some(device.clone());
            device
        }
    };

    let mut manager_guard = FASTSWAP_MANAGER.lock().unwrap();
    let manager = match manager_guard.as_mut() {
        Some(m) => m,
        None => {
            *manager_guard = Some(FastSwapManager::new(53317));
            manager_guard.as_mut().unwrap()
        }
    };

    if let Err(e) = manager.start(local_device).await {
        tracing::error!("[FastSwap] Failed to start on demand: {}", e);
    } else {
        tracing::info!("[FastSwap] Started on demand");
    }
}

/// Get global progress tracker for UI access
pub fn get_progress_tracker() -> ProgressTracker {
    Arc::clone(&GLOBAL_PROGRESS_TRACKER)
}

/// Add pending transfer for approval
pub async fn add_pending_transfer(transfer: PendingTransfer) {
    let mut pending = PENDING_TRANSFERS.write().await;
    pending.push(transfer);
}

/// Get all pending transfers
pub async fn get_pending_transfers() -> Vec<PendingTransfer> {
    PENDING_TRANSFERS.read().await.clone()
}

/// Approve a transfer
pub async fn approve_transfer(session_id: &str) {
    let mut approved = APPROVED_SESSIONS.write().await;
    approved.push(session_id.to_string());

    // Remove from pending
    let mut pending = PENDING_TRANSFERS.write().await;
    pending.retain(|t| t.session_id != session_id);

    // Notify the waiting server handler
    if let Some(notify) = APPROVAL_NOTIFIERS.write().await.remove(session_id) {
        notify.notify_one();
    }
}

/// Deny a transfer
pub async fn deny_transfer(session_id: &str) {
    let mut pending = PENDING_TRANSFERS.write().await;
    pending.retain(|t| t.session_id != session_id);

    // Notify the waiting server handler (will see it was denied)
    if let Some(notify) = APPROVAL_NOTIFIERS.write().await.remove(session_id) {
        notify.notify_one();
    }
}

/// Check if transfer is approved
pub async fn is_transfer_approved(session_id: &str) -> bool {
    let approved = APPROVED_SESSIONS.read().await;
    approved.contains(&session_id.to_string())
}

impl Drop for FastSwapManager {
    fn drop(&mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_approve_transfer_notifies_watcher() {
        let session_id = "test-session-1";
        let notify = register_approval_watch(session_id).await;

        let notified = {
            let notify = notify.clone();
            let sid = session_id.to_string();
            tokio::spawn(async move {
                notify.notified().await;
                is_transfer_approved(&sid).await
            })
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        approve_transfer(session_id).await;

        let approved = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            notified,
        )
        .await
        .expect("timeout waiting for notification")
        .expect("task panicked");

        assert!(approved, "transfer should be approved");
    }

    #[tokio::test]
    async fn test_deny_transfer_notifies_watcher() {
        let session_id = "test-session-2";
        let notify = register_approval_watch(session_id).await;

        let notified = {
            let notify = notify.clone();
            let sid = session_id.to_string();
            tokio::spawn(async move {
                notify.notified().await;
                is_transfer_approved(&sid).await
            })
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        deny_transfer(session_id).await;

        let approved = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            notified,
        )
        .await
        .expect("timeout waiting for notification")
        .expect("task panicked");

        assert!(!approved, "transfer should not be approved after denial");
    }

    #[tokio::test]
    async fn test_approve_then_check_pending_removed() {
        let session_id = "test-session-3";

        let pending = PendingTransfer {
            session_id: session_id.to_string(),
            sender_name: "TestSender".into(),
            sender_device: "TestDevice".into(),
            file_count: 2,
            total_size: 1024,
            files: vec!["a.txt".into(), "b.txt".into()],
        };
        add_pending_transfer(pending).await;

        approve_transfer(session_id).await;

        let pending_list = get_pending_transfers().await;
        assert!(
            !pending_list.iter().any(|t| t.session_id == session_id),
            "approved transfer should be removed from pending"
        );
    }
}
