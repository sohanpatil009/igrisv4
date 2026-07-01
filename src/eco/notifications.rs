use crate::eco::constants::*;
use crate::eco::device::EcoDevice;
use crate::eco::discovery::DiscoveredDevice;
use crate::eco::errors::EcoResult;
use crate::eco::events::{EcoEvent, EventBus};
use crate::eco::protocol::{NotificationReplyPayload, NotificationSyncPayload};
use crate::eco::storage::{EcoStorage, NotificationEntry};
use crate::eco::transport::EcoTransport;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

lazy_static::lazy_static! {
    static ref NOTIFICATION_LIST: std::sync::Mutex<Vec<NotificationEntry>> = std::sync::Mutex::new(Vec::new());
    static ref NOTIFICATION_MANAGER: std::sync::Mutex<Option<Arc<NotificationManager>>> = std::sync::Mutex::new(None);
}

pub fn get_notification_list() -> Vec<NotificationEntry> {
    NOTIFICATION_LIST.lock().map(|g| g.clone()).unwrap_or_default()
}

pub fn mark_notification_read(id: &str) {
    if let Ok(mut guard) = NOTIFICATION_LIST.lock() {
        if let Some(entry) = guard.iter_mut().find(|e| e.id == id) {
            entry.read = true;
        }
    }
}

pub fn mark_notification_replied(id: &str) {
    if let Ok(mut guard) = NOTIFICATION_LIST.lock() {
        if let Some(entry) = guard.iter_mut().find(|e| e.id == id) {
            entry.replied = true;
        }
    }
}

fn push_notification(entry: NotificationEntry) {
    if let Ok(mut guard) = NOTIFICATION_LIST.lock() {
        guard.push(entry);
        if guard.len() > NOTIFICATION_HISTORY_MAX {
            guard.remove(0);
        }
    }
}

pub struct NotificationManager {
    event_bus: Arc<EventBus>,
    transport: Arc<EcoTransport>,
    known_devices: Arc<RwLock<HashMap<String, DiscoveredDevice>>>,
    storage: Arc<std::sync::Mutex<EcoStorage>>,
}

impl NotificationManager {
    pub(crate) fn new(
        event_bus: Arc<EventBus>,
        transport: Arc<EcoTransport>,
        known_devices: Arc<RwLock<HashMap<String, DiscoveredDevice>>>,
        storage: Arc<std::sync::Mutex<EcoStorage>>,
    ) -> Self {
        Self {
            event_bus,
            transport,
            known_devices,
            storage,
        }
    }

    pub fn start(&self) {
        let transport = self.transport.clone();
        let known_devices = self.known_devices.clone();
        let storage = self.storage.clone();

        let handler: Arc<dyn Fn(EcoEvent) + Send + Sync> = Arc::new(move |event| {
            match event {
                EcoEvent::NotificationReceived(payload) => {
                    let storage = storage.clone();
                    let entry = NotificationEntry {
                        id: uuid::Uuid::new_v4().to_string(),
                        notification_id: payload.notification_id.clone(),
                        app_name: payload.app_name.clone(),
                        title: payload.title.clone(),
                        body: payload.body.clone(),
                        source_device: payload.source_device.clone(),
                        source_device_name: payload.source_device_name.clone(),
                        timestamp: payload.timestamp,
                        reply_allowed: payload.reply_allowed,
                        reply_id: payload.reply_id.clone(),
                        replied: false,
                        read: false,
                    };
                    push_notification(entry.clone());
                    if let Ok(mut guard) = storage.lock() {
                        let _ = guard.add_notification_entry(entry);
                    };
                }
                EcoEvent::NotificationReplyReceived(payload) => {
                    println!("[ECO] Reply received for notification {}: {}", payload.notification_id, payload.message);
                }
                _ => {}
            }
        });

        self.event_bus.subscribe(handler);
    }

    pub async fn broadcast_notification(
        &self,
        app_name: &str,
        title: &str,
        body: &str,
        reply_allowed: bool,
        reply_id: Option<String>,
    ) {
        let payload = NotificationSyncPayload {
            notification_id: uuid::Uuid::new_v4().to_string(),
            app_name: app_name.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            source_device: String::new(),
            source_device_name: String::new(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            reply_allowed,
            reply_id,
        };

        let peers = self.known_devices.read().await;
        for (_id, discovered) in peers.iter() {
            if let Some(addr) = &discovered.device.addr {
                let _ = self.transport.send_notification(addr, &payload).await;
            }
        }
    }

    pub async fn send_reply(
        &self,
        notification_id: &str,
        reply_id: &str,
        message: &str,
        target_device: &str,
        target_device_name: &str,
    ) {
        let payload = NotificationReplyPayload {
            notification_id: notification_id.to_string(),
            reply_id: reply_id.to_string(),
            message: message.to_string(),
            target_device: target_device.to_string(),
            target_device_name: target_device_name.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        let peers = self.known_devices.read().await;
        for (_id, discovered) in peers.iter() {
            if discovered.device.id.to_string() == target_device {
                if let Some(addr) = &discovered.device.addr {
                    let _ = self.transport.send_notification_reply(addr, &payload).await;
                }
                break;
            }
        }
    }
}

pub fn init_manager(manager: Arc<NotificationManager>) {
    if let Ok(mut guard) = NOTIFICATION_MANAGER.lock() {
        *guard = Some(manager);
    }
}

pub async fn send_notification_reply(
    notification_id: &str,
    reply_id: &str,
    message: &str,
    target_device: &str,
    target_device_name: &str,
) {
    if let Ok(guard) = NOTIFICATION_MANAGER.lock() {
        if let Some(ref manager) = *guard {
            manager.send_reply(notification_id, reply_id, message, target_device, target_device_name).await;
        }
    }
}
