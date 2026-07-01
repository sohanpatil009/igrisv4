use crate::eco::constants::*;
use crate::eco::errors::{EcoError, EcoResult};
use crate::eco::events::{EcoEvent, EventBus};
use crate::eco::storage::{ClipboardEntry, EcoStorage};
use crate::platform::ecosystem::PlatformClipboard;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ClipboardData {
    pub content: String,
    pub content_type: String,
    pub content_hash: String,
    pub source_device: String,
    pub timestamp: i64,
}

pub struct ClipboardManager {
    platform: Box<dyn PlatformClipboard>,
    event_bus: Arc<EventBus>,
    storage: Arc<std::sync::Mutex<EcoStorage>>,
    last_content_hash: Option<String>,
    last_applied_hash: Option<String>,
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
        }
    }

    pub async fn start_monitoring(manager: Arc<std::sync::Mutex<Self>>) {
        let mut interval = tokio::time::interval(
            std::time::Duration::from_secs(CLIPBOARD_POLL_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            let event = {
                let mut guard = match manager.lock() {
                    Ok(g) => g,
                    Err(_) => continue,
                };
                let text = match guard.platform.get_text() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                if text.is_empty() { continue; }
                let hash = hash_content(&text);

                let is_own_change = Some(&hash) == guard.last_applied_hash.as_ref();
                let is_unchanged = Some(&hash) == guard.last_content_hash.as_ref();

                if is_own_change || is_unchanged {
                    continue;
                }

                println!("[ECO] Clipboard changed: hash={}", &hash[..16]);

                let data = ClipboardData {
                    content: text.clone(),
                    content_type: "text/plain".to_string(),
                    content_hash: hash.clone(),
                    source_device: String::new(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                };

                let entry = ClipboardEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    content: text,
                    content_type: "text/plain".to_string(),
                    source_device: String::new(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    content_hash: hash.clone(),
                };

                if let Ok(mut storage) = guard.storage.lock() {
                    let _ = storage.add_clipboard_entry(entry);
                }

                guard.last_content_hash = Some(hash);
                let event = EcoEvent::ClipboardChanged(Arc::new(data));
                event
            };
            let bus = manager.lock().ok().map(|g| g.event_bus.clone());
            if let Some(bus) = bus {
                bus.emit(event);
            }
        }
    }

    pub fn apply_clipboard(&mut self, data: &ClipboardData) -> EcoResult<()> {
        if self.last_content_hash.as_ref() == Some(&data.content_hash) {
            return Ok(());
        }

        self.platform.set_text(&data.content)?;
        self.last_applied_hash = Some(data.content_hash.clone());
        self.last_content_hash = Some(data.content_hash.clone());

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
