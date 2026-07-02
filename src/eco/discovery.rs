use crate::eco::constants::*;
use crate::eco::device::EcoDevice;
use crate::eco::events::{EcoEvent, EventBus};
use crate::eco::protocol::{ClipboardSyncPayload, NotificationSyncPayload, NotificationReplyPayload};
use axum::extract::ConnectInfo;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoveredEcoDevice {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub ip: String,
    pub port: u16,
    pub is_trusted: bool,
    pub is_online: bool,
    pub last_seen_secs: u64,
}

#[derive(Deserialize)]
struct PairRequestPayload {
    sender_id: String,
    sender_name: String,
    #[serde(default = "default_port")]
    sender_port: u16,
    otp_hash: String,
}

fn default_port() -> u16 { ECO_TLS_PORT }

#[derive(Deserialize)]
struct PairVerifyPayload {
    pending_id: String,
    otp: String,
    remote_device_id: String,
}

#[derive(Deserialize)]
struct UntrustPayload {
    device_id: String,
}

lazy_static::lazy_static! {
    pub static ref ECO_NETWORK_DEVICES: Arc<RwLock<Vec<DiscoveredEcoDevice>>> =
        Arc::new(RwLock::new(Vec::new()));
    pub static ref PENDING_PAIRING: Arc<RwLock<Vec<PendingPair>>> =
        Arc::new(RwLock::new(Vec::new()));
}

#[derive(Clone, Debug)]
pub struct PendingPair {
    pub id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_addr: SocketAddr,
    pub otp_hash: String,
    pub received_at: Instant,
}

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
        let pair_pending = PENDING_PAIRING.clone();
        let local_id = crate::eco::pairing::get_local_device_id();
        let local_name = crate::eco::pairing::get_local_device_name();

        let app = axum::Router::new()
            .route("/api/ecosystem/v1/info", axum::routing::get({
                let dev_id = local_id.clone();
                let dev_name = local_name.clone();
                move || {
                    let did = dev_id.clone();
                    let dnm = dev_name.clone();
                    async move {
                        axum::Json(serde_json::json!({
                            "status": "ok",
                            "ecosystem": true,
                            "clipboard_sync": true,
                            "device_id": did,
                            "device_name": dnm,
                        }))
                    }
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
            }))
            .route("/api/ecosystem/v1/pair/request", axum::routing::post({
                let pending = pair_pending.clone();
                move |ConnectInfo(remote_addr): ConnectInfo<SocketAddr>, body: axum::extract::Json<PairRequestPayload>| {
                    let pending = pending.clone();
                    async move {
                        let p = body.0;
                        let id = uuid::Uuid::new_v4().to_string();
                        let entry = PendingPair {
                            id: id.clone(),
                            sender_id: p.sender_id,
                            sender_name: p.sender_name,
                            sender_addr: SocketAddr::new(remote_addr.ip(), p.sender_port),
                            otp_hash: p.otp_hash,
                            received_at: Instant::now(),
                        };
                        let mut list = pending.write().await;
                        list.push(entry);
                        axum::Json(serde_json::json!({"status": "ok", "pending_id": id}))
                    }
                }
            }))
            .route("/api/ecosystem/v1/pair/verify", axum::routing::post({
                let pending = pair_pending.clone();
                let bus = event_bus.clone();
                move |body: axum::extract::Json<PairVerifyPayload>| {
                    let pending = pending.clone();
                    let bus = bus.clone();
                    async move {
                        let p = body.0;
                        let mut list = pending.write().await;
                        let pos = list.iter().position(|r| r.id == p.pending_id);
                        if let Some(idx) = pos {
                            let req = list.remove(idx);
                            let entered_hash = crate::eco::pairing::hash_otp_code(&p.otp);
                            if entered_hash == req.otp_hash {
                                let mut eco_network = ECO_NETWORK_DEVICES.write().await;
                                for dev in eco_network.iter_mut() {
                                    if dev.id == p.remote_device_id {
                                        dev.is_trusted = true;
                                    }
                                }
                                bus.emit(EcoEvent::DeviceTrusted(
                                    std::sync::Arc::new(crate::eco::device::EcoDevice::new(p.remote_device_id.clone()))
                                ));
                                axum::Json(serde_json::json!({
                                    "status": "ok",
                                    "trusted": true,
                                    "initiator_id": req.sender_id
                                }))
                            } else {
                                axum::Json(serde_json::json!({"status": "ok", "trusted": false}))
                            }
                        } else {
                            axum::Json(serde_json::json!({"status": "error", "message": "Pending request not found"}))
                        }
                    }
                }
            }))
            .route("/api/ecosystem/v1/pair/untrust", axum::routing::post({
                move |body: axum::extract::Json<UntrustPayload>| {
                    async move {
                        let p = body.0;
                        let mut eco_network = ECO_NETWORK_DEVICES.write().await;
                        for dev in eco_network.iter_mut() {
                            if dev.id == p.device_id {
                                dev.is_trusted = false;
                            }
                        }
                        axum::Json(serde_json::json!({"status": "ok"}))
                    }
                }
            }))
            .route("/api/ecosystem/v1/notification/sync", axum::routing::post({
                let bus = event_bus.clone();
                move |body: axum::extract::Json<NotificationSyncPayload>| {
                    let bus = bus.clone();
                    async move {
                        let payload = body.0;
                        let notif = crate::eco::notification::NotificationData {
                            id: payload.notification_id,
                            app_name: payload.app_name,
                            title: payload.title,
                            body: payload.body,
                            device_name: payload.source_device_name,
                            device_id: payload.source_device_id,
                            timestamp: payload.timestamp,
                            read: false,
                            replied: false,
                        };
                        bus.emit(EcoEvent::NotificationReceived(
                            notif,
                            "remote".to_string(),
                        ));
                        axum::Json(serde_json::json!({"status": "ok"}))
                    }
                }
            }))
            .route("/api/ecosystem/v1/notification/reply", axum::routing::post({
                let bus = event_bus.clone();
                move |body: axum::extract::Json<NotificationReplyPayload>| {
                    let bus = bus.clone();
                    async move {
                        let payload = body.0;
                        let reply = crate::eco::notification::NotificationReply {
                            notification_id: payload.notification_id,
                            reply_text: payload.reply_text,
                            source_device_id: payload.source_device_id,
                        };
                        bus.emit(EcoEvent::NotificationReplied(reply));
                        axum::Json(serde_json::json!({"status": "ok"}))
                    }
                }
            }));

        let bind_addr = *addr;
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(bind_addr).await;
            if let Ok(listener) = listener {
                axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.ok();
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
                let mut network_list = ECO_NETWORK_DEVICES.write().await;
                for peer in peers {
                    let id = peer.id.to_string();
                    let now = Instant::now();
                    let is_trusted = false;
                    match devices.get_mut(&id) {
                        Some(existing) => {
                            existing.last_heartbeat = now;
                        }
                        None => {
                            devices.insert(id.clone(), DiscoveredDevice {
                                device: peer.clone(),
                                last_heartbeat: now,
                            });
                        }
                    }
                    let ip_str = peer.addr.map(|a| a.ip().to_string()).unwrap_or_default();
                    let port = peer.addr.map(|a| a.port()).unwrap_or(ECO_TLS_PORT);
                    let discovered = DiscoveredEcoDevice {
                        id: id.clone(),
                        name: peer.name.clone(),
                        hostname: peer.hostname.clone(),
                        ip: ip_str,
                        port,
                        is_trusted,
                        is_online: true,
                        last_seen_secs: 0,
                    };
                    if !network_list.iter().any(|d| d.id == id) {
                        network_list.push(discovered);
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
            if let Ok(probe_ip) = ip.parse::<std::net::IpAddr>() {
                if probe_ip == *local_ip {
                    continue;
                }
            }
            let client = client.clone();
            let task = tokio::spawn(async move {
                let url = format!("https://{}:{}/api/ecosystem/v1/info", ip, ECO_TLS_PORT);
                match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                            let device_id = body.get("device_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let device_name = body.get("device_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&format!("eco-{}", ip))
                                .to_string();
                            if device_id.is_empty() {
                                return None;
                            }
                            match uuid::Uuid::parse_str(&device_id) {
                                Ok(uuid) => {
                                    let mut eco = EcoDevice::new(device_name);
                                    eco.id = uuid;
                                    if let Ok(ip_addr) = ip.parse::<std::net::IpAddr>() {
                                        eco.addr = Some(SocketAddr::new(ip_addr, ECO_TLS_PORT));
                                    }
                                    eco.capabilities.clipboard_sync = true;
                                    Some(eco)
                                }
                                Err(_) => None,
                            }
                        } else {
                            None
                        }
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