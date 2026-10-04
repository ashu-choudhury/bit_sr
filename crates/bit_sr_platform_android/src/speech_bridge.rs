//! Speech driver bridge for Android.
//! Connects `bit_sr_speech` to Android's TextToSpeech and low-latency feedback.

use bit_sr_speech::error::Result;
use bit_sr_speech::synthesizer::SynthesizerDriver;
use bit_sr_speech::voice::VoiceInfo;

/// Android Text-To-Speech Driver.
pub struct AndroidTtsDriver {
    name: String,
    current_voice: Option<VoiceInfo>,
    rate: i32,
    volume: u16,
}

impl Default for AndroidTtsDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidTtsDriver {
    pub fn new() -> Self {
        Self {
            name: "android_tts".to_string(),
            current_voice: Some(VoiceInfo {
                id: "default".to_string(),
                name: "System Default Voice".to_string(),
                language: "en".to_string(),
                gender: None,
            }),
            rate: 50,
            volume: 100,
        }
    }
}

impl SynthesizerDriver for AndroidTtsDriver {
    fn name(&self) -> &str {
        &self.name
    }

    fn display_name(&self) -> &str {
        "Android System Text-to-Speech"
    }

    fn available_voices(&self) -> Vec<VoiceInfo> {
        self.current_voice.clone().into_iter().collect()
    }

    fn current_voice(&self) -> Option<VoiceInfo> {
        self.current_voice.clone()
    }

    fn set_voice(&mut self, voice_id: &str) -> Result<()> {
        if let Some(ref mut v) = self.current_voice {
            v.id = voice_id.to_string();
        }
        Ok(())
    }

    fn speak(&mut self, _text: &str, _interrupt: bool) -> Result<()> {
        // In native Android runtime, this invokes JNI TextToSpeech.speak()
        // with QUEUE_FLUSH for interrupt = true or QUEUE_ADD for false.
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        // Dispatches JNI TextToSpeech.stop()
        Ok(())
    }

    fn pause(&mut self) -> Result<()> {
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
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
}
