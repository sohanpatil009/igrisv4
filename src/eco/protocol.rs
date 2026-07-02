use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CURRENT_VERSION: &str = "1.0.0";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EcoMessage {
    pub version: String,
    pub msg_type: MessageType,
    pub sender_id: String,
    pub sender_name: String,
    pub payload: serde_json::Value,
    pub timestamp: i64,
    pub signature: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessageType {
    Announcement(DeviceAnnouncement),
    ClipboardSync(ClipboardSyncPayload),
    ClipboardPullRequest(ClipboardPullRequest),
    ClipboardPullResponse(ClipboardPullResponse),
    NotificationSync(NotificationSyncPayload),
    NotificationReply(NotificationReplyPayload),
    PairingRequest(PairingPayload),
    PairingResponse(PairingPayload),
    Heartbeat,
    Ack,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceAnnouncement {
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub hostname: String,
    pub version: String,
    pub capabilities: HashMap<String, bool>,
    pub public_key: Option<String>,
    pub eco_port: u16,
    pub ip_address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardSyncPayload {
    pub content_hash: String,
    pub content: String,
    pub content_type: String,
    pub source_device: String,
    pub timestamp: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardPullRequest {
    pub content_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardPullResponse {
    pub content_hash: String,
    pub content: String,
    pub content_type: String,
    pub success: bool,
}

/// Notification synced from a remote device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationSyncPayload {
    pub notification_id: String,
    pub app_name: String,
    pub title: String,
    pub body: String,
    pub source_device_id: String,
    pub source_device_name: String,
    pub timestamp: i64,
}

/// Reply to a notification sent back to the originating device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationReplyPayload {
    pub notification_id: String,
    pub reply_text: String,
    pub source_device_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairingPayload {
    pub device_id: String,
    pub device_name: String,
    pub public_key: Option<String>,
    pub accepted: bool,
    pub message: Option<String>,
}

impl EcoMessage {
    pub fn new(msg_type: MessageType, sender_id: String, sender_name: String) -> Self {
        let payload = serde_json::to_value(&msg_type).unwrap_or_default();
        Self {
            version: CURRENT_VERSION.to_string(),
            msg_type,
            sender_id,
            sender_name,
            payload,
            timestamp: chrono::Utc::now().timestamp_millis(),
            signature: None,
        }
    }
}
