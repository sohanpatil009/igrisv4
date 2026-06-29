use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: String,
    pub file_name: String,
    pub size: u64,
    pub file_type: String,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareUploadRequest {
    pub info: DeviceInfo,
    pub files: Vec<FileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub alias: String,
    pub version: String,
    #[serde(rename = "deviceModel")]
    pub device_model: String,
    #[serde(rename = "deviceType")]
    pub device_type: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareUploadResponse {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub files: Vec<FileResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResponse {
    pub id: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmUploadRequest {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmUploadResponse {
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct TransferState {
    pub session_id: String,
    pub files: Vec<FileTransfer>,
    pub total_size: u64,
    pub transferred: u64,
    pub status: TransferStatus,
    pub confirmed: bool,
}

#[derive(Debug, Clone)]
pub struct FileTransfer {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub transferred: u64,
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransferStatus {
    Preparing,
    Transferring,
    Completed,
    Failed(String),
    Cancelled,
}
