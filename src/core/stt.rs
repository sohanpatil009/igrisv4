use sherpa_onnx::{
    OfflineRecognizer, OfflineRecognizerConfig,
    OfflineSenseVoiceModelConfig,
};

const SAMPLE_RATE: i32 = 16000;

pub struct SttEngine {
    recognizer: OfflineRecognizer,
}

impl SttEngine {
    pub fn transcribe(&self, audio_samples: &[f32]) -> Result<String, Box<dyn std::error::Error>> {
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(SAMPLE_RATE, audio_samples);
        self.recognizer.decode(&stream);
        let text = stream
            .get_result()
            .map(|r| r.text)
            .unwrap_or_default();
        Ok(text.trim().to_string())
    }
}

pub fn init_stt_engine() -> Result<SttEngine, Box<dyn std::error::Error>> {
    let model_path = "pkg/models/sense-voice/model.onnx";
    let tokens_path = "pkg/models/sense-voice/tokens.txt";

    if !std::path::Path::new(model_path).exists() {
        return Err("SenseVoice model not found at pkg/models/sense-voice/. Please run setup first.".into());
    }

    println!("[STT] Loading SenseVoice model: {}", model_path);

    let mut config = OfflineRecognizerConfig::default();
    config.model_config.sense_voice = OfflineSenseVoiceModelConfig {
        model: Some(model_path.into()),
        language: Some("auto".into()),
        use_itn: true,
    };
    config.model_config.tokens = Some(tokens_path.into());
    config.model_config.num_threads = 2;

    let recognizer = OfflineRecognizer::create(&config)
        .ok_or("Failed to create sherpa-onnx offline recognizer")?;

    println!("[STT] SenseVoice ready");
    Ok(SttEngine { recognizer })
}

pub fn transcribe_audio(
    audio_samples: &[f32],
    engine: &SttEngine,
) -> Result<String, Box<dyn std::error::Error>> {
    engine.transcribe(audio_samples)
}

pub async fn hybrid_transcribe_audio(
    audio_samples: &[f32],
    engine: &SttEngine,
) -> Result<String, Box<dyn std::error::Error>> {
    engine.transcribe(audio_samples)
}
