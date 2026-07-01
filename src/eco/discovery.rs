use crate::eco::constants::*;
use crate::eco::device::{Capabilities, DeviceStatus, EcoDevice};
use crate::eco::events::{EcoEvent, EventBus};
use crate::eco::protocol::ClipboardSyncPayload;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

pub(crate) struct DiscoveredDevice {
    pub(crate) device: EcoDevice,
    pub(crate) last_heartbeat: Instant,
}

pub struct DeviceDiscovery {
    known_devices: Arc<RwLock<HashMap<String, DiscoveredDevice>>>,
    event_bus: Arc<EventBus>,
}

impl DeviceDiscovery {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            known_devices: Arc::new(RwLock::new(HashMap::new())),
            event_bus,
        }
    }

    pub(crate) fn get_known_devices(&self) -> Arc<RwLock<HashMap<String, DiscoveredDevice>>> {
        self.known_devices.clone()
    }

    /// Start the ecosystem's own HTTP server on the given address.
    /// Receives clipboard sync payloads on POST /api/ecosystem/v1/clipboard/sync
    /// and emits them via the event bus.
    pub async fn start_server(&self, addr: &SocketAddr) {
        let event_bus = self.event_bus.clone();

        let app = axum::Router::new()
            .route("/api/ecosystem/v1/info", axum::routing::get({
                move || async move {
                    axum::Json(serde_json::json!({
                        "status": "ok",
                        "ecosystem": true,
                        "clipboard_sync": true,
                    }))
                }
            }))
            .route("/api/ecosystem/v1/clipboard/sync", axum::routing::post({
                let bus = event_bus.clone();
                move |body: axum::extract::Json<ClipboardSyncPayload>| {
                    let bus = bus.clone();
                    async move {
                        let payload = body.0;
                        let data = crate::eco::clipboard::ClipboardData {
                            content: payload.content,
                            content_type: payload.content_type,
                            content_hash: payload.content_hash,
                            source_device: payload.source_device,
                            timestamp: payload.timestamp,
                        };
                        bus.emit(EcoEvent::ClipboardReceived(
                            std::sync::Arc::new(data),
                            String::new(),
                        ));
                        axum::Json(serde_json::json!({"status": "ok"}))
                    }
                }
            }));

        let bind_addr = *addr;
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(bind_addr).await;
            if let Ok(listener) = listener {
                axum::serve(listener, app).await.ok();
            }
        });
    }

    /// Periodically scan the subnet via HTTPS on the ecosystem TLS port (53328)
    /// to discover peers with clipboard-sync capability (FastSwap technique).
    pub async fn start_discovery(&self) {
        let known_devices = self.known_devices.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(HEARTBEAT_INTERVAL_SECS)
            );
            loop {
                interval.tick().await;
                let local_ip = local_ip_address::local_ip()
                    .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 1)));
                let peers = Self::scan_subnet_https(&local_ip).await;
                let mut devices = known_devices.write().await;
                for peer in peers {
                    let id = peer.id.to_string();
                    let now = Instant::now();
                    match devices.get_mut(&id) {
                        Some(existing) => {
                            existing.last_heartbeat = now;
                        }
                        None => {
                            devices.insert(id.clone(), DiscoveredDevice {
                                device: peer,
                                last_heartbeat: now,
                            });
                        }
                    }
                }
            }
        });
    }

    /// HTTPS subnet scan — same technique FastSwap uses.
    /// Probes each IP on the ecosystem TLS port (53328).
    async fn scan_subnet_https(local_ip: &std::net::IpAddr) -> Vec<EcoDevice> {
        let subnet = match local_ip {
            std::net::IpAddr::V4(v4) => {
                let octets = v4.octets();
                format!("{}.{}.{}", octets[0], octets[1], octets[2])
            }
            _ => return Vec::new(),
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap_or_default();

        let mut discovered = Vec::new();
        let mut tasks = Vec::new();

        for i in 1..=254 {
            let ip = format!("{}.{}", subnet, i);
            let client = client.clone();
            let task = tokio::spawn(async move {
                let url = format!("https://{}:{}/api/ecosystem/v1/info", ip, ECO_TLS_PORT);
                match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        let mut eco = EcoDevice::new(format!("eco-{}", ip));
                        if let Ok(ip_addr) = ip.parse::<std::net::IpAddr>() {
                            eco.addr = Some(SocketAddr::new(ip_addr, ECO_TLS_PORT));
                        }
                        eco.capabilities.clipboard_sync = true;
                        Some(eco)
                    }
                    _ => None,
                }
            });
            tasks.push(task);
            if tasks.len() >= 50 {
                for task in tasks.drain(..) {
                    if let Ok(Some(device)) = task.await {
                        discovered.push(device);
                    }
                }
            }
        }
        for task in tasks {
            if let Ok(Some(device)) = task.await {
                discovered.push(device);
            }
        }
        discovered
    }

    pub async fn run_cleanup(&self) {
        let known_devices = self.known_devices.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(DEVICE_TIMEOUT_SECS / 2)
            );
            loop {
                interval.tick().await;
                let mut devices = known_devices.write().await;
                let now = Instant::now();
                devices.retain(|_, d| {
                    now.duration_since(d.last_heartbeat).as_secs() < DEVICE_TIMEOUT_SECS
                });
            }
        });
    }
}