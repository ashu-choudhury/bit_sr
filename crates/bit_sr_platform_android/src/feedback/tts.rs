//! Android Text-to-Speech (TTS) synthesizer driver.
//! Connects `bit_sr_speech` to Android's `TextToSpeech` service.

use bit_sr_speech::error::Result;
use bit_sr_speech::synthesizer::SynthesizerDriver;
use bit_sr_speech::voice::VoiceInfo;
use parking_lot::RwLock;

type SpeechCallback = Box<dyn Fn(&str, bool) + Send + Sync>;
static SPEECH_CALLBACK: RwLock<Option<SpeechCallback>> = RwLock::new(None);

/// Sets the global speech callback invoked on speak/stop.
pub fn set_speech_callback<F>(f: F)
where
    F: Fn(&str, bool) + Send + Sync + 'static,
{
    *SPEECH_CALLBACK.write() = Some(Box::new(f));
}

/// Clears the global speech callback.
pub fn clear_speech_callback() {
    *SPEECH_CALLBACK.write() = None;
}

/// Android Text-To-Speech Driver.
pub struct AndroidTtsDriver {
    name: String,
    current_voice: Option<VoiceInfo>,
    rate: i32,
    volume: u16,
    last_spoken: String,
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
            last_spoken: String::new(),
        }
    }

    /// Last string sent to speak.
    pub fn last_spoken(&self) -> &str {
        &self.last_spoken
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

    fn speak(&mut self, text: &str, interrupt: bool) -> Result<()> {
        self.last_spoken = text.to_string();
        log::debug!("Android TTS speak: {}", text);
        if let Some(ref cb) = *SPEECH_CALLBACK.read() {
            cb(text, interrupt);
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.last_spoken.clear();
        log::debug!("Android TTS stop");
        if let Some(ref cb) = *SPEECH_CALLBACK.read() {
            cb("", true);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_android_tts_driver() {
        let mut tts = AndroidTtsDriver::new();
        assert_eq!(tts.name(), "android_tts");
        assert!(tts.speak("Hello Android", true).is_ok());
        assert_eq!(tts.last_spoken(), "Hello Android");
        assert!(tts.stop().is_ok());
        assert_eq!(tts.last_spoken(), "");
    }
}
