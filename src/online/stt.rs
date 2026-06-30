// src/online/stt.rs - Online STT using NVIDIA NIM Parakeet ASR

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

const SAMPLE_RATE: u32 = 16000;

#[derive(Debug, Clone)]
pub struct OnlineStt {
    client: Client,
    api_key: String,
    base_url: String,
}

#[derive(Serialize)]
struct ParakeetRequest {
    audio: String, // base64 encoded audio
    sample_rate: u32,
    language: Option<String>,
    model: String,
}

#[derive(Deserialize, Debug)]
struct ParakeetResponse {
    text: String,
    language: Option<String>,
    duration: Option<f32>,
}

#[derive(Deserialize, Debug)]
struct ErrorResponse {
    error: Option<ErrorDetail>,
}

#[derive(Deserialize, Debug)]
struct ErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
    code: Option<String>,
}

impl OnlineStt {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let api_key = env::var("NVIDIA_API_KEY")
            .map_err(|_| "NVIDIA_API_KEY not set in .env")?;

        let base_url = env::var("NVIDIA_NIM_PARAKEET_BASE_URL")
            .unwrap_or_else(|_| "https://integrate.api.nvidia.com/v1".to_string());

        Ok(Self {
            client: Client::new(),
            api_key,
            base_url,
        })
    }

    /// Transcribe audio samples (f32, 16kHz mono) using NVIDIA NIM Parakeet
    pub async fn transcribe(&self, audio_samples: &[f32]) -> Result<String, Box<dyn std::error::Error>> {
        // Convert f32 samples to 16-bit PCM bytes
        let pcm_bytes: Vec<u8> = audio_samples
            .iter()
            .flat_map(|&s| {
                let sample = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                sample.to_le_bytes()
            })
            .collect();

        // Base64 encode
        let audio_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &pcm_bytes);

        let request = ParakeetRequest {
            audio: audio_b64,
            sample_rate: SAMPLE_RATE,
            language: Some("en".to_string()),
            model: "parakeet-tdt-0.6b-v2".to_string(),
        };

        let url = format!("{}/audio/transcriptions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            let error_msg = if let Ok(err) = serde_json::from_str::<ErrorResponse>(&error_text) {
                err.error.map(|e| e.message).unwrap_or(error_text)
            } else {
                error_text
            };
            return Err(format!("Parakeet API error ({}): {}", status, error_msg).into());
        }

        let result: ParakeetResponse = response.json().await?;
        Ok(result.text.trim().to_string())
    }
}

impl Default for OnlineStt {
    fn default() -> Self {
        Self::new().expect("Failed to create OnlineStt - check API key in .env")
    }
}

/// Transcribe audio using online Parakeet ASR (NVIDIA NIM)
pub async fn transcribe_online(audio_samples: &[f32]) -> Result<String, Box<dyn std::error::Error>> {
    let stt = OnlineStt::new()?;
    stt.transcribe(audio_samples).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_online_stt() {
        let stt = OnlineStt::new().unwrap();
        // Generate 1 second of silence
        let silence: Vec<f32> = vec![0.0; 16000];
        let result = stt.transcribe(&silence).await;
        println!("Result: {:?}", result);
    }
}