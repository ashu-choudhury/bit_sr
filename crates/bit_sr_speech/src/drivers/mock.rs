//! Mock Synthesizer Driver for Testing and Headless Environments.

use crate::error::Result;
use crate::synthesizer::SynthesizerDriver;
use crate::voice::{VoiceGender, VoiceInfo};
use std::sync::{Arc, Mutex};

/// A mock synthesizer that captures spoken text into a buffer without audio hardware.
#[derive(Debug, Clone)]
pub struct MockSynthesizer {
    spoken_history: Arc<Mutex<Vec<String>>>,
    current_voice: Option<VoiceInfo>,
    rate: i32,
    volume: u16,
    pitch: i32,
    is_paused: bool,
}

impl MockSynthesizer {
    pub fn new() -> Self {
        let default_voice = VoiceInfo {
            id: "mock_en_voice".to_string(),
            name: "Mock English Voice".to_string(),
            language: "en-US".to_string(),
            gender: Some(VoiceGender::Neutral),
        };

        Self {
            spoken_history: Arc::new(Mutex::new(Vec::new())),
            current_voice: Some(default_voice),
            rate: 0,
            volume: 100,
            pitch: 0,
            is_paused: false,
        }
    }

    /// Returns a copy of all strings spoken since creation or last clear.
    pub fn get_spoken_history(&self) -> Vec<String> {
        self.spoken_history.lock().unwrap().clone()
    }

    /// Clears the captured speech history.
    pub fn clear_history(&self) {
        self.spoken_history.lock().unwrap().clear();
    }
}

impl Default for MockSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SynthesizerDriver for MockSynthesizer {
    fn name(&self) -> &str {
        "mock"
    }

    fn display_name(&self) -> &str {
        "Mock Test Synthesizer"
    }

    fn available_voices(&self) -> Vec<VoiceInfo> {
        vec![
            VoiceInfo {
                id: "mock_en_voice".to_string(),
                name: "Mock English Voice".to_string(),
                language: "en-US".to_string(),
                gender: Some(VoiceGender::Neutral),
            },
            VoiceInfo {
                id: "mock_es_voice".to_string(),
                name: "Mock Spanish Voice".to_string(),
                language: "es-ES".to_string(),
                gender: Some(VoiceGender::Female),
            },
        ]
    }

    fn current_voice(&self) -> Option<VoiceInfo> {
        self.current_voice.clone()
    }

    fn set_voice(&mut self, voice_id: &str) -> Result<()> {
        let voices = self.available_voices();
        if let Some(v) = voices.into_iter().find(|v| v.id == voice_id) {
            self.current_voice = Some(v);
            Ok(())
        } else {
            Err(crate::error::SpeechError::VoiceNotFound(voice_id.to_string()))
        }
    }

    fn speak(&mut self, text: &str, interrupt: bool) -> Result<()> {
        let mut guard = self.spoken_history.lock().unwrap();
        if interrupt {
            // Instant speech interrupt behavior in mock
        }
        guard.push(text.to_string());
        log::debug!("[MockSynth speak] (interrupt={}) \"{}\"", interrupt, text);
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        log::debug!("[MockSynth stop]");
        Ok(())
    }

    fn pause(&mut self) -> Result<()> {
        self.is_paused = true;
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        self.is_paused = false;
        Ok(())
    }

    fn set_rate(&mut self, rate: i32) -> Result<()> {
        self.rate = rate;
        Ok(())
    }

    fn get_rate(&self) -> i32 {
        self.rate
    }

    fn set_volume(&mut self, volume: u16) -> Result<()> {
        self.volume = volume;
        Ok(())
    }

    fn get_volume(&self) -> u16 {
        self.volume
    }

    fn set_pitch(&mut self, pitch: i32) -> Result<()> {
        self.pitch = pitch;
        Ok(())
    }

    fn get_pitch(&self) -> i32 {
        self.pitch
    }
}
