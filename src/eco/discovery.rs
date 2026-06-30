use crate::eco::constants::*;
use crate::eco::device::{Capabilities, DeviceStatus, EcoDevice};
use crate::eco::errors::{EcoError, EcoResult};
use crate::eco::protocol::{DeviceAnnouncement, EcoMessage, MessageType};
use crate::eco::transport::EcoTransport;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

pub struct DeviceDiscovery {
    local_device: Arc<RwLock<EcoDevice>>,
    known_devices: Arc<RwLock<HashMap<String, DiscoveredDevice>>>,
    transport: Arc<EcoTransport>,
    eco_port: u16,
}

struct DiscoveredDevice {
    device: EcoDevice,
    last_heartbeat: Instant,
}

impl DeviceDiscovery {
    pub fn new(
        local_device: Arc<RwLock<EcoDevice>>,
        transport: Arc<EcoTransport>,
        eco_port: u16,
    ) -> Self {
        Self {
            local_device,
            known_devices: Arc::new(RwLock::new(HashMap::new())),
            transport,
            eco_port,
        }
    }

    pub fn get_known_devices(&self) -> Arc<RwLock<HashMap<String, DiscoveredDevice>>> {
        self.known_devices.clone()
    }

    pub async fn start_broadcast(&self) {
        let local_device = self.local_device.clone();
        let transport = self.transport.clone();
        let eco_port = self.eco_port;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(HEARTBEAT_INTERVAL_SECS)
            );
            loop {
                interval.tick().await;
                let device = local_device.read().await;
                let announcement = DeviceAnnouncement {
                    device_id: device.id.to_string(),
                    device_name: device.name.clone(),
                    platform: device.platform.clone(),
                    hostname: device.hostname.clone(),
                    version: device.version.clone(),
                    capabilities: [
                        ("clipboard_sync".to_string(), device.capabilities.clipboard_sync),
                        ("notification_sync".to_string(), device.capabilities.notification_sync),
                        ("remote_commands".to_string(), device.capabilities.remote_commands),
                    ].into(),
                    public_key: device.public_key.clone(),
                    eco_port,
                };
                drop(device);

                let msg = Arc::new(EcoMessage::new(
                    MessageType::Announcement(announcement),
                    String::new(),
                    String::new(),
                ));

                let _ = Self::broadcast_message(transport.clone(), eco_port, msg).await;
            }
        });
    }

    pub async fn start_listener(&self, addr: &SocketAddr) {
        let known_devices = self.known_devices.clone();
        let transport = self.transport.clone();
        let local_device = self.local_device.clone();
        let bind_addr = *addr;

        let app = axum::Router::new()
            .route("/api/ecosystem/v1/info", axum::routing::get({
                let local_device = local_device.clone();
                move || {
                    let device = local_device.clone();
                    async move {
                        let d = device.read().await;
                        let announcement = DeviceAnnouncement {
                            device_id: d.id.to_string(),
                            device_name: d.name.clone(),
                            platform: d.platform.clone(),
                            hostname: d.hostname.clone(),
                            version: d.version.clone(),
                            capabilities: [
                                ("clipboard_sync".to_string(), d.capabilities.clipboard_sync),
                                ("notification_sync".to_string(), d.capabilities.notification_sync),
                                ("remote_commands".to_string(), d.capabilities.remote_commands),
                            ].into(),
                            public_key: d.public_key.clone(),
                            eco_port: 0,
                        };
                        let msg = EcoMessage::new(
                            MessageType::Announcement(announcement),
                            d.id.to_string(),
                            d.name.clone(),
                        );
                        axum::Json(msg)
                    }
                }
            }))
            .route("/api/ecosystem/v1/message", axum::routing::post({
                let known_devices = known_devices.clone();
                move |body: axum::extract::Json<EcoMessage>| {
                    let known = known_devices.clone();
                    async move {
                        let msg = body.0;
                        match &msg.msg_type {
                            MessageType::Announcement(a) => {
                                Self::handle_announcement(&known, a, msg.sender_id.clone()).await;
                            }
                            _ => {}
                        }
                        axum::Json(serde_json::json!({"status": "ok"}))
                    }
                }
            }));

        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(bind_addr).await;
            if let Ok(listener) = listener {
                axum::serve(listener, app).await.ok();
            }
        });
    }

    async fn handle_announcement(
        known_devices: &Arc<RwLock<HashMap<String, DiscoveredDevice>>>,
        announcement: &DeviceAnnouncement,
        _sender_id: String,
    ) {
        let mut device = EcoDevice::new(announcement.device_name.clone());
        device.id = uuid::Uuid::parse_str(&announcement.device_id).unwrap_or_default();
        device.platform = announcement.platform.clone();
        device.hostname = announcement.hostname.clone();
        device.version = announcement.version.clone();
        device.capabilities = Capabilities {
            clipboard_sync: announcement.capabilities.get("clipboard_sync").copied().unwrap_or(false),
            notification_sync: announcement.capabilities.get("notification_sync").copied().unwrap_or(false),
            remote_commands: announcement.capabilities.get("remote_commands").copied().unwrap_or(false),
        };
        device.status = DeviceStatus::Online;
        device.public_key = announcement.public_key.clone();
        device.touch();

        let mut devices = known_devices.write().await;
        devices.insert(announcement.device_id.clone(), DiscoveredDevice {
            device,
            last_heartbeat: Instant::now(),
        });
    }

    async fn broadcast_message(
        transport: Arc<EcoTransport>,
        eco_port: u16,
        message: Arc<EcoMessage>,
    ) {
        let local_ip = local_ip_address::local_ip().unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
        let subnet = match local_ip {
            std::net::IpAddr::V4(v4) => {
                let octets = v4.octets();
                format!("{}.{}.{}", octets[0], octets[1], octets[2])
            }
            _ => "192.168.1".to_string(),
        };

        let mut tasks = Vec::new();
        for i in 1..=254 {
            let ip = format!("{}.{}", subnet, i);
            let transport = transport.clone();
            let message = message.clone();
            let addr_str = format!("{}:{}", ip, eco_port);
            if let Ok(addr) = addr_str.parse::<std::net::SocketAddr>() {
                tasks.push(tokio::spawn(async move {
                    let _ = transport.send_message(&addr, &message).await;
                }));
            }
            if tasks.len() >= 50 {
                for task in tasks.drain(..) {
                    task.await.ok();
                }
            }
        }
        for task in tasks.drain(..) {
            task.await.ok();
        }
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
