use tonic::transport::{Channel, Endpoint};
use tonic::metadata::MetadataValue;

fn init_rustls_crypto() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("Failed to install rustls CryptoProvider (aws-lc-rs)");
        println!("[Parakeet STT] rustls CryptoProvider initialized (aws-lc-rs)");
    });
}

const SAMPLE_RATE: u32 = 16000;
const FUNCTION_ID: &str = "d3fe9151-442b-4204-a70d-5fcc597fd610";
const GRPC_ENDPOINT: &str = "https://grpc.nvcf.nvidia.com:443";

pub mod nvidia {
    pub mod riva {
        tonic::include_proto!("nvidia.riva");
        pub mod asr {
            tonic::include_proto!("nvidia.riva.asr");
        }
    }
}

use nvidia::riva::asr::riva_speech_recognition_client::RivaSpeechRecognitionClient;
use nvidia::riva::asr::RecognizeRequest;
use nvidia::riva::asr::RecognitionConfig;
use nvidia::riva::AudioEncoding;

pub struct OnlineStt {
    client: RivaSpeechRecognitionClient<Channel>,
    api_key: String,
}

impl OnlineStt {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        init_rustls_crypto();
        println!("[Parakeet STT] Initializing gRPC client...");

        let api_key = std::env::var("NVIDIA_API_KEY")
            .map_err(|e| {
                println!("[Parakeet STT] ERROR: NVIDIA_API_KEY not set: {}", e);
                "NVIDIA_API_KEY not set in .env"
            })?;

        println!("[Parakeet STT] Connecting to {} ...", GRPC_ENDPOINT);

        let endpoint = Endpoint::new(GRPC_ENDPOINT.to_string())?;
        let channel = endpoint
            .connect_timeout(std::time::Duration::from_secs(10))
            .connect()
            .await
            .map_err(|e| {
                println!("[Parakeet STT] ERROR connecting gRPC channel: {:#}", e);
                e
            })?;

        println!("[Parakeet STT] gRPC channel connected");

        let client = RivaSpeechRecognitionClient::new(channel);

        Ok(Self { client, api_key })
    }

    pub async fn transcribe(&mut self, audio_samples: &[f32]) -> Result<String, Box<dyn std::error::Error>> {
        let duration_ms = (audio_samples.len() as f64 / SAMPLE_RATE as f64) * 1000.0;
        println!("[Parakeet STT] Transcribing {:.0}ms of audio ({} samples)", duration_ms, audio_samples.len());

        let audio_bytes: Vec<u8> = audio_samples
            .iter()
            .flat_map(|&s| {
                let sample = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                sample.to_le_bytes().to_vec()
            })
            .collect();

        println!("[Parakeet STT] Raw PCM: {} bytes ({} samples)", audio_bytes.len(), audio_samples.len());

        let request = RecognizeRequest {
            config: Some(RecognitionConfig {
                encoding: AudioEncoding::LinearPcm as i32,
                sample_rate_hertz: SAMPLE_RATE as i32,
                language_code: "en-US".to_string(),
                audio_channel_count: 1,
                enable_automatic_punctuation: true,
                model: String::new(),
                max_alternatives: 1,
                profanity_filter: false,
                speech_contexts: vec![],
                enable_word_time_offsets: false,
                enable_separate_recognition_per_channel: false,
                verbatim_transcripts: true,
                diarization_config: None,
                custom_configuration: std::collections::HashMap::new(),
                ..Default::default()
            }),
            audio: audio_bytes,
            id: None,
        };

        println!("[Parakeet STT] gRPC Recognize via {} (function-id: {})", GRPC_ENDPOINT, FUNCTION_ID);

        let mut tonic_req = tonic::Request::new(request);
        tonic_req.metadata_mut().insert(
            "authorization",
            MetadataValue::try_from(&format!("Bearer {}", self.api_key))?,
        );
        tonic_req.metadata_mut().insert(
            "function-id",
            MetadataValue::try_from(FUNCTION_ID)?,
        );

        let response = self.client.recognize(tonic_req).await?;
        let response = response.into_inner();

        println!("[Parakeet STT] Got {} result(s)", response.results.len());

        if let Some(result) = response.results.first() {
            if let Some(alt) = result.alternatives.first() {
                let transcript = alt.transcript.trim().to_string();
                println!("[Parakeet STT] Transcription: \"{}\"", transcript);
                return Ok(transcript);
            }
        }

        println!("[Parakeet STT] No transcription in response");
        Ok(String::new())
    }
}

pub async fn transcribe_online(audio_samples: &[f32]) -> Result<String, Box<dyn std::error::Error>> {
    let mut stt = OnlineStt::new().await?;
    stt.transcribe(audio_samples).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_grpc_connect() {
        init_rustls_crypto();
        println!("[TEST] Connecting...");
        match Endpoint::new(GRPC_ENDPOINT.to_string())
            .unwrap()
            .connect_timeout(std::time::Duration::from_secs(15))
            .connect()
            .await
        {
            Ok(_) => println!("[TEST] gRPC channel connected OK"),
            Err(e) => println!("[TEST] Connection error: {:#}", e),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_online_stt() {
        let mut stt = match OnlineStt::new().await {
            Ok(s) => s,
            Err(e) => {
                println!("[TEST] OnlineStt::new failed: {:#}", e);
                return;
            }
        };
        let silence: Vec<f32> = vec![0.0; 16000];
        match stt.transcribe(&silence).await {
            Ok(t) => println!("[TEST] Transcription: {}", t),
            Err(e) => println!("[TEST] Transcribe error: {:#}", e),
        }
    }
}
