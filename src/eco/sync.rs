use crate::eco::clipboard::ClipboardData;
use crate::eco::device::EcoDevice;
use crate::eco::discovery::DiscoveredDevice;
use crate::eco::errors::EcoResult;
use crate::eco::events::{EcoEvent, EventBus};
use crate::eco::protocol::ClipboardSyncPayload;
use crate::eco::transport::EcoTransport;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SyncManager {
    event_bus: Arc<EventBus>,
    transport: Arc<EcoTransport>,
    known_devices: Arc<RwLock<HashMap<String, DiscoveredDevice>>>,
    local_device: Arc<RwLock<EcoDevice>>,
}

impl SyncManager {
    pub(crate) fn new(
        event_bus: Arc<EventBus>,
        transport: Arc<EcoTransport>,
        known_devices: Arc<RwLock<HashMap<String, DiscoveredDevice>>>,
        local_device: Arc<RwLock<EcoDevice>>,
    ) -> Self {
        Self {
            event_bus,
            transport,
            known_devices,
            local_device,
        }
    }

    pub(crate) async fn start(&self) -> EcoResult<()> {
        let transport = self.transport.clone();
        let known_devices = self.known_devices.clone();
        let local_device = self.local_device.clone();

        let handler: Arc<dyn Fn(EcoEvent) + Send + Sync> = Arc::new(move |event| {
            if let EcoEvent::ClipboardChanged(data) = event {
                let transport = transport.clone();
                let known_devices = known_devices.clone();
                let local_device = local_device.clone();
                tokio::spawn(async move {
                    let peers = known_devices.read().await;
                    let device = local_device.read().await;
                    let sender_id = device.id.to_string();
                    let sender_name = device.name.clone();
                    drop(device);

                    let payload = ClipboardSyncPayload {
                        content_hash: data.content_hash.clone(),
                        content: data.content.clone(),
                        content_type: data.content_type.clone(),
                        source_device: sender_id.clone(),
                        timestamp: chrono::Utc::now().timestamp_millis(),
                        image_data: data.image_data.clone(),
                    };

                    let peer_count = peers.iter().filter(|(_, d)| d.device.capabilities.clipboard_sync && d.device.addr.is_some()).count();
                    println!("[ECO] Broadcasting clipboard to {} peer(s)", peer_count);

                    for (_id, discovered) in peers.iter() {
                        if !discovered.device.capabilities.clipboard_sync {
                            continue;
                        }
                        if let Some(addr) = &discovered.device.addr {
                            println!("[ECO] Sending clipboard to {}", addr);
                            let _ = transport.send_clipboard(addr, &payload).await;
                        }
                    }
                });
            }
        });

        self.event_bus.subscribe(handler);
        Ok(())
    }

    pub async fn shutdown(&self) {
    }
}