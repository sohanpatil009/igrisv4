use crate::eco::constants::*;
use crate::eco::errors::{EcoError, EcoResult};
use crate::eco::protocol::EcoMessage;
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

    pub async fn send_message(
        &self,
        addr: &SocketAddr,
        message: &EcoMessage,
    ) -> EcoResult<EcoMessage> {
        let url = format!("http://{}/api/ecosystem/v1/message", addr);
        let resp = self.http_client
            .post(&url)
            .json(message)
            .send()
            .await
            .map_err(|e| EcoError::Transport(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EcoError::Transport(format!(
                "Peer returned status {}", resp.status()
            )));
        }

        resp.json().await.map_err(|e| EcoError::Transport(e.to_string()))
    }

    pub async fn fetch_clipboard(
        &self,
        addr: &SocketAddr,
        content_hash: &str,
    ) -> EcoResult<String> {
        let url = format!(
            "http://{}/api/ecosystem/v1/clipboard/{}",
            addr, content_hash
        );
        let resp = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| EcoError::Transport(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EcoError::Transport(format!(
                "Clipboard fetch returned {}", resp.status()
            )));
        }

        resp.text().await.map_err(|e| EcoError::Transport(e.to_string()))
    }

    pub async fn probe_device(&self, addr: &str, port: u16) -> EcoResult<EcoMessage> {
        let url = format!("http://{}:{}/api/ecosystem/v1/info", addr, port);
        let resp = self.http_client
            .get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .map_err(|e| EcoError::Transport(e.to_string()))?;

        resp.json().await.map_err(|e| EcoError::Transport(e.to_string()))
    }
}
