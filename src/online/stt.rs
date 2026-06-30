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
    audio: AudioData,
    config: RecognitionConfig,
}

#[derive(Serialize)]
struct AudioData {
    content: String, // base64 encoded WAV audio
}

#[derive(Serialize)]
struct RecognitionConfig {
    language_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    encoding: String,
    sample_rate_hertz: u32,
    audio_channel_count: u32,
    enable_automatic_punctuation: bool,
}

#[derive(Deserialize, Debug)]
struct ParakeetResponse {
    #[serde(default)]
    text: String,
    #[serde(default)]
    results: Vec<RecognitionResult>,
}

#[derive(Deserialize, Debug)]
struct RecognitionResult {
    #[serde(default)]
    alternatives: Vec<RecognitionAlternative>,
}

#[derive(Deserialize, Debug)]
struct RecognitionAlternative {
    #[serde(default)]
    transcript: String,
}

#[derive(Deserialize, Debug)]
struct ErrorResponse {
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

impl std::fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.detail.as_deref().or(self.message.as_deref()).unwrap_or("unknown error"))
    }
}

impl OnlineStt {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let api_key = env::var("NVIDIA_API_KEY")
            .map_err(|_| "NVIDIA_API_KEY not set in .env")?;

        let base_url = env::var("NVIDIA_NIM_PARAKEET_BASE_URL")
            .unwrap_or_else(|_| "https://api.nvcf.nvidia.com/v2/nvcf/pexec/functions/d3fe9151-442b-4204-a70d-5fcc597fd610".to_string());

        Ok(Self {
            client: Client::new(),
            api_key,
            base_url,
        })
    }

    /// Build a WAV file in memory from f32 PCM samples (16-bit mono, 16kHz)
    fn build_wav(&self, audio_samples: &[f32]) -> Vec<u8> {
        let sample_rate = SAMPLE_RATE as u32;
        let bits_per_sample: u16 = 16;
        let num_channels: u16 = 1;
        let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
        let block_align = num_channels * bits_per_sample / 8;
        let data_size = audio_samples.len() as u32 * 2; // 16-bit = 2 bytes per sample
        let file_size = 36 + data_size;

        let mut wav = Vec::with_capacity(file_size as usize);

        // RIFF header
        wav.extend(b"RIFF");
        wav.extend(&file_size.to_le_bytes());
        wav.extend(b"WAVE");

        // fmt chunk
        wav.extend(b"fmt ");
        wav.extend(&16u32.to_le_bytes()); // chunk size
        wav.extend(&1u16.to_le_bytes());  // PCM format
        wav.extend(&num_channels.to_le_bytes());
        wav.extend(&sample_rate.to_le_bytes());
        wav.extend(&byte_rate.to_le_bytes());
        wav.extend(&block_align.to_le_bytes());
        wav.extend(&bits_per_sample.to_le_bytes());

        // data chunk
        wav.extend(b"data");
        wav.extend(&data_size.to_le_bytes());

        // PCM data
        for &s in audio_samples {
            let sample = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            wav.extend(&sample.to_le_bytes());
        }

        wav
    }

    /// Transcribe audio samples (f32, 16kHz mono) using NVIDIA NIM Parakeet
    pub async fn transcribe(&self, audio_samples: &[f32]) -> Result<String, Box<dyn std::error::Error>> {
        let duration_ms = (audio_samples.len() as f64 / SAMPLE_RATE as f64) * 1000.0;
        println!("[Parakeet STT] Transcribing {:.0}ms of audio ({} samples)", duration_ms, audio_samples.len());

        // Build WAV file in memory
        let wav_bytes = self.build_wav(audio_samples);
        println!("[Parakeet STT] WAV file size: {} bytes", wav_bytes.len());

        let audio_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &wav_bytes);
        println!("[Parakeet STT] Base64 encoded: {} bytes", audio_b64.len());

        // Send as Riva ASR REST JSON format
        let url = self.base_url.clone();
        let request = ParakeetRequest {
            audio: AudioData {
                content: audio_b64,
            },
            config: RecognitionConfig {
                language_code: "en-US".to_string(),
                model: Some("parakeet-tdt-0.6b-v2".to_string()),
                encoding: "LINEAR16_PCM".to_string(),
                sample_rate_hertz: SAMPLE_RATE,
                audio_channel_count: 1,
                enable_automatic_punctuation: true,
            },
        };

        println!("[Parakeet STT] POST {} (Riva REST JSON)", url);
        println!("[Parakeet STT] Config: lang={}, model={}, rate={}Hz",
            request.config.language_code,
            request.config.model.as_deref().unwrap_or("default"),
            request.config.sample_rate_hertz);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        println!("[Parakeet STT] Response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            let snippet = if error_text.len() > 500 { &error_text[..500] } else { &error_text };
            println!("[Parakeet STT] ERROR: {} (body: {})", status, snippet);
            return Err(format!("Parakeet API error ({}): {}", status, snippet).into());
        }

        let result = response.text().await?;
        println!("[Parakeet STT] Raw response: {}", &result[..result.len().min(500)]);

        // Parse Riva response format
        if let Ok(parsed) = serde_json::from_str::<ParakeetResponse>(&result) {
            if !parsed.text.is_empty() {
                println!("[Parakeet STT] Transcription: \"{}\"", parsed.text.trim());
                return Ok(parsed.text.trim().to_string());
            }
            if let Some(result) = parsed.results.first() {
                if let Some(alt) = result.alternatives.first() {
                    println!("[Parakeet STT] Transcription: \"{}\"", alt.transcript.trim());
                    return Ok(alt.transcript.trim().to_string());
                }
            }
        }

        // Try parsing as plain text
        let trimmed = result.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('{') {
            println!("[Parakeet STT] Plain text: {}", trimmed);
            Ok(trimmed.to_string())
        } else {
            println!("[Parakeet STT] Could not parse response: {}", trimmed);
            Ok(trimmed.to_string())
        }
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