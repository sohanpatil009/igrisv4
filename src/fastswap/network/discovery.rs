use crate::fastswap::models::Device;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

const HTTP_PORT: u16 = 53317;
const TLS_PORT_START: u16 = 53318;
const TLS_PORT_END: u16 = 53328;

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
                // Probe HTTP first, then HTTPS for TLS-enabled devices
                let mut device = Self::probe_device_http(&ip, HTTP_PORT).await;
                if device.is_none() {
                    for tls_port in TLS_PORT_START..TLS_PORT_END {
                        device = Self::probe_device_https(&ip, tls_port).await;
                        if device.is_some() {
                            break;
                        }
                    }
                }
                device
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

    async fn probe_device_http(ip: &str, port: u16) -> Option<Device> {
        let url = format!("http://{}:{}/api/localsend/v2/info", ip, port);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .ok()?;

        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<Device>().await {
                        Ok(mut device) => {
                            device.ip = ip.to_string();
                            device.port = port;
                            tracing::info!("Found device: {} at {}:{} (HTTP)", device.alias, ip, port);
                            return Some(device);
                        }
                        Err(e) => {
                            tracing::debug!("Device at {} returned invalid JSON: {}", ip, e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::debug!("No HTTP response from {}:{}: {}", ip, port, e);
            }
        }

        None
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
                            tracing::debug!("Device at {} returned invalid JSON: {}", ip, e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::debug!("No HTTPS response from {}:{}: {}", ip, port, e);
            }
        }

        None
    }

    pub async fn get_devices(&self) -> Vec<Device> {
        self.devices.read().await.clone()
    }
}
