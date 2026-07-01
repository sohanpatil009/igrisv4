use crate::eco::constants::*;
use crate::eco::errors::{EcoError, EcoResult};
use crate::eco::protocol::{ClipboardSyncPayload, NotificationSyncPayload, NotificationReplyPayload};
use std::net::SocketAddr;

pub struct EcoTransport {
    http_client: reqwest::Client,
}

impl EcoTransport {
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(PEER_REQUEST_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();
        Self { http_client }
    }

    /// Send clipboard payload to a peer via HTTPS (FastSwap TLS port).
    pub async fn send_clipboard(
        &self,
        addr: &SocketAddr,
        payload: &ClipboardSyncPayload,
    ) -> EcoResult<()> {
        let url = format!("https://{}/api/ecosystem/v1/clipboard/sync", addr);
        let resp = self.http_client
            .post(&url)
            .json(payload)
            .send()
            .await
            .map_err(|e| EcoError::Transport(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EcoError::Transport(format!(
                "Peer returned status {}", resp.status()
            )));
        }

        Ok(())
    }

    /// Send notification to a peer via HTTPS.
    pub async fn send_notification(
        &self,
        addr: &SocketAddr,
        payload: &NotificationSyncPayload,
    ) -> EcoResult<()> {
        let url = format!("https://{}/api/ecosystem/v1/notification/sync", addr);
        let resp = self.http_client
            .post(&url)
            .json(payload)
            .send()
            .await
            .map_err(|e| EcoError::Transport(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EcoError::Transport(format!(
                "Peer returned status {}", resp.status()
            )));
        }

        Ok(())
    }

    /// Send notification reply to a peer via HTTPS.
    pub async fn send_notification_reply(
        &self,
        addr: &SocketAddr,
        payload: &NotificationReplyPayload,
    ) -> EcoResult<()> {
        let url = format!("https://{}/api/ecosystem/v1/notification/reply", addr);
        let resp = self.http_client
            .post(&url)
            .json(payload)
            .send()
            .await
            .map_err(|e| EcoError::Transport(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EcoError::Transport(format!(
                "Peer returned status {}", resp.status()
            )));
        }

        Ok(())
    }
}