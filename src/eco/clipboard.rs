use crate::eco::constants::*;
use crate::eco::errors::{EcoError, EcoResult};
use crate::eco::events::{EcoEvent, EventBus};
use crate::eco::storage::{ClipboardEntry, EcoStorage};
use crate::platform::ecosystem::PlatformClipboard;
use base64::Engine;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ClipboardData {
    pub content: String,
    pub content_type: String,
    pub content_hash: String,
    pub source_device: String,
    pub timestamp: i64,
    pub image_data: Option<String>,
}

pub struct ClipboardManager {
    platform: Box<dyn PlatformClipboard>,
    event_bus: Arc<EventBus>,
    storage: Arc<std::sync::Mutex<EcoStorage>>,
    last_content_hash: Option<String>,
    last_applied_hash: Option<String>,
    last_image_hash: Option<String>,
}

impl ClipboardManager {
    pub fn new(
        platform: Box<dyn PlatformClipboard>,
        event_bus: Arc<EventBus>,
        storage: Arc<std::sync::Mutex<EcoStorage>>,
    ) -> Self {
        Self {
            platform,
            event_bus,
            storage,
            last_content_hash: None,
            last_applied_hash: None,
            last_image_hash: None,
        }
    }

    pub async fn start_monitoring(manager: Arc<std::sync::Mutex<Self>>) {
        let mut interval = tokio::time::interval(
            std::time::Duration::from_secs(CLIPBOARD_POLL_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            let (text_event, image_event) = {
                let mut guard = match manager.lock() {
                    Ok(g) => g,
                    Err(_) => continue,
                };

                let mut text_event = None;
                let mut image_event = None;

                // Check text clipboard
                if let Ok(text) = guard.platform.get_text() {
                    if !text.is_empty() {
                        let hash = hash_content(&text);
                        let is_own_change = Some(&hash) == guard.last_applied_hash.as_ref();
                        let is_unchanged = Some(&hash) == guard.last_content_hash.as_ref();
                        if !is_own_change && !is_unchanged {
                            println!("[ECO] Clipboard text changed: hash={}", &hash[..16]);

                            let data = ClipboardData {
                                content: text.clone(),
                                content_type: "text/plain".to_string(),
                                content_hash: hash.clone(),
                                source_device: String::new(),
                                timestamp: chrono::Utc::now().timestamp_millis(),
                                image_data: None,
                            };

                            let entry = ClipboardEntry {
                                id: uuid::Uuid::new_v4().to_string(),
                                content: text,
                                content_type: "text/plain".to_string(),
                                source_device: String::new(),
                                timestamp: chrono::Utc::now().timestamp_millis(),
                                content_hash: hash.clone(),
                                image_data: None,
                            };

                            if let Ok(mut storage) = guard.storage.lock() {
                                let _ = storage.add_clipboard_entry(entry);
                            }

                            guard.last_content_hash = Some(hash);
                            text_event = Some(EcoEvent::ClipboardChanged(Arc::new(data)));
                        }
                    }
                }

                // Check image clipboard
                if let Ok(Some(image_bytes)) = guard.platform.get_image() {
                    let image_hash = hash_bytes(&image_bytes);
                    let is_own_change = Some(&image_hash) == guard.last_applied_hash.as_ref();
                    let is_unchanged = Some(&image_hash) == guard.last_image_hash.as_ref();
                    if !is_own_change && !is_unchanged {
                        println!("[ECO] Clipboard image changed: hash={}", &image_hash[..16]);

                        let b64 = base64::engine::general_purpose::STANDARD.encode(&image_bytes);

                        let data = ClipboardData {
                            content: String::new(),
                            content_type: "image/png".to_string(),
                            content_hash: image_hash.clone(),
                            source_device: String::new(),
                            timestamp: chrono::Utc::now().timestamp_millis(),
                            image_data: Some(b64.clone()),
                        };

                        let entry = ClipboardEntry {
                            id: uuid::Uuid::new_v4().to_string(),
                            content: String::new(),
                            content_type: "image/png".to_string(),
                            source_device: String::new(),
                            timestamp: chrono::Utc::now().timestamp_millis(),
                            content_hash: image_hash.clone(),
                            image_data: Some(b64),
                        };

                        if let Ok(mut storage) = guard.storage.lock() {
                            let _ = storage.add_clipboard_entry(entry);
                        }

                        guard.last_image_hash = Some(image_hash);
                        image_event = Some(EcoEvent::ClipboardChanged(Arc::new(data)));
                    }
                }

                (text_event, image_event)
            };

            let bus = manager.lock().ok().map(|g| g.event_bus.clone());
            if let Some(bus) = bus {
                if let Some(ev) = text_event {
                    bus.emit(ev);
                }
                if let Some(ev) = image_event {
                    bus.emit(ev);
                }
            }
        }
    }

    pub fn apply_clipboard(&mut self, data: &ClipboardData) -> EcoResult<()> {
        if self.last_content_hash.as_ref() == Some(&data.content_hash)
            || self.last_image_hash.as_ref() == Some(&data.content_hash)
        {
            return Ok(());
        }

        if data.content_type.starts_with("image/") {
            if let Some(b64) = &data.image_data {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|e| EcoError::Clipboard(e.to_string()))?;
                self.platform.set_image(&bytes)?;
                self.last_applied_hash = Some(data.content_hash.clone());
                self.last_image_hash = Some(data.content_hash.clone());
            }
        } else {
            self.platform.set_text(&data.content)?;
            self.last_applied_hash = Some(data.content_hash.clone());
            self.last_content_hash = Some(data.content_hash.clone());
        }

        let arc = Arc::new(data.clone());
        self.event_bus.emit(EcoEvent::ClipboardApplied(arc));
        Ok(())
    }

    pub fn get_current_hash(&self) -> Option<String> {
        self.last_content_hash.clone()
    }
}

pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn hash_bytes(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}
