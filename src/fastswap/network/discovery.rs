use crate::fastswap::models::Device;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

const TLS_PORT: u16 = 53318;

pub struct DiscoveryService {
    devices: Arc<RwLock<Vec<Device>>>,
}

impl DiscoveryService {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn scan_network(&self, local_ip: &str) -> Result<Vec<Device>> {
        tracing::info!("Starting network scan from {}", local_ip);

        let mut discovered = Vec::new();

        let parts: Vec<&str> = local_ip.split('.').collect();
        if parts.len() != 4 {
            return Ok(discovered);
        }

        let subnet = format!("{}.{}.{}", parts[0], parts[1], parts[2]);

        let mut tasks = Vec::new();

        for i in 1..=254 {
            let ip = format!("{}.{}", subnet, i);
            if ip == local_ip {
                continue;
            }

            let task = tokio::spawn(async move {
                Self::probe_device_https(&ip, TLS_PORT).await
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

        let mut devices = self.devices.write().await;
        *devices = discovered.clone();

        tracing::info!("Scan complete. Found {} devices", discovered.len());
        Ok(discovered)
    }

    async fn probe_device_https(ip: &str, port: u16) -> Option<Device> {
        let url = format!("https://{}:{}/api/localsend/v2/info", ip, port);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .danger_accept_invalid_certs(true)
            .build()
            .ok()?;

        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<Device>().await {
                        Ok(mut device) => {
                            device.ip = ip.to_string();
                            device.port = port;
                            tracing::info!("Found device: {} at {}:{} (TLS)", device.alias, ip, port);
                            return Some(device);
                        }
                        Err(e) => {
                            tracing::trace!("Device at {} returned invalid JSON: {}", ip, e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::trace!("No HTTPS response from {}:{}: {}", ip, port, e);
            }
        }

        None
    }

    pub async fn get_devices(&self) -> Vec<Device> {
        self.devices.read().await.clone()
    }
}
