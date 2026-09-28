//! Central Speech Hub and Driver Dispatcher.
//! Manages active synthesizers, utterance queueing, speech interruption,
//! and dynamic extension registration.

use crate::error::{Result, SpeechError};
use crate::synthesizer::{SpeechPriority, SynthesizerDriver};
use crate::voice::VoiceInfo;
use std::collections::HashMap;

/// The central speech management engine for the screen reader.
pub struct SpeechHub {
    drivers: HashMap<String, Box<dyn SynthesizerDriver>>,
    active_driver_name: Option<String>,
}

impl SpeechHub {
    /// Creates an empty speech hub with no drivers registered.
    pub fn new() -> Self {
        Self {
            drivers: HashMap::new(),
            active_driver_name: None,
        }
    }

    /// Initializes a speech hub pre-populated with standard platform drivers.
    /// On Windows, registers SAPI 5 and Mock drivers, activating SAPI 5 by default.
    pub fn default_platform() -> Result<Self> {
        let mut hub = Self::new();

        // Always register mock driver
        hub.register_driver(Box::new(crate::drivers::MockSynthesizer::new()));

        #[cfg(windows)]
        {
            match crate::drivers::Sapi5Synthesizer::new() {
                Ok(sapi) => {
                    let name = sapi.name().to_string();
                    hub.register_driver(Box::new(sapi));
                    let _ = hub.set_active_synthesizer(&name);
                    log::info!("SAPI 5 initialized as default speech driver");
                }
                Err(e) => {
                    log::warn!("Could not initialize SAPI 5 ({:?}); falling back to mock driver", e);
                    let _ = hub.set_active_synthesizer("mock");
                }
            }
        }

        #[cfg(not(windows))]
        {
            let _ = hub.set_active_synthesizer("mock");
        }

        Ok(hub)
    }

    /// Registers a new synthesizer driver (built-in, native, or WebAssembly extension).
    pub fn register_driver(&mut self, driver: Box<dyn SynthesizerDriver>) {
        let name = driver.name().to_string();
        if self.active_driver_name.is_none() {
            self.active_driver_name = Some(name.clone());
        }
        self.drivers.insert(name, driver);
    }

    /// Returns a list of all registered synthesizer drivers: (id, display_name).
    pub fn available_synthesizers(&self) -> Vec<(&str, &str)> {
        self.drivers
            .values()
            .map(|d| (d.name(), d.display_name()))
            .collect()
    }

    /// Sets the active synthesizer driver by ID.
    pub fn set_active_synthesizer(&mut self, name: &str) -> Result<()> {
        if self.drivers.contains_key(name) {
            self.active_driver_name = Some(name.to_string());
            Ok(())
        } else {
            Err(SpeechError::SynthNotAvailable(name.to_string()))
        }
    }

    /// Returns the name of the currently active synthesizer driver.
    pub fn active_synthesizer_name(&self) -> Option<&str> {
        self.active_driver_name.as_deref()
    }

    /// Returns a reference to the active synthesizer driver.
    pub fn active_driver(&self) -> Option<&dyn SynthesizerDriver> {
        let name = self.active_driver_name.as_ref()?;
        self.drivers.get(name).map(|b| b.as_ref())
    }

    /// Returns a mutable reference to the active synthesizer driver.
    pub fn active_driver_mut(&mut self) -> Option<&mut (dyn SynthesizerDriver + 'static)> {
        let name = self.active_driver_name.as_ref()?;
        self.drivers.get_mut(name).map(|b| b.as_mut())
    }

    /// Returns available voices on the currently active synthesizer.
    pub fn available_voices(&self) -> Vec<VoiceInfo> {
        self.active_driver()
            .map(|d| d.available_voices())
            .unwrap_or_default()
    }

    /// Returns the currently active voice on the active synthesizer.
    pub fn current_voice(&self) -> Option<VoiceInfo> {
        self.active_driver().and_then(|d| d.current_voice())
    }

    /// Sets the active voice by ID on the active synthesizer.
    pub fn set_voice(&mut self, voice_id: &str) -> Result<()> {
        if let Some(driver) = self.active_driver_mut() {
            driver.set_voice(voice_id)
        } else {
            Err(SpeechError::SynthNotAvailable("No active synthesizer".to_string()))
        }
    }

    /// Speaks an utterance with the specified priority.
    pub fn speak(&mut self, text: &str, priority: SpeechPriority) -> Result<()> {
        let interrupt = matches!(priority, SpeechPriority::Now);
        if let Some(driver) = self.active_driver_mut() {
            driver.speak(text, interrupt)
        } else {
            Err(SpeechError::SynthNotAvailable("No active synthesizer".to_string()))
        }
    }

    /// Instantly halts all speech and silences the active synthesizer.
    /// Triggered immediately on key down (`SpeechInterrupt`).
    pub fn interrupt(&mut self) -> Result<()> {
        if let Some(driver) = self.active_driver_mut() {
            driver.stop()
        } else {
            Ok(())
        }
    }

    /// Stops speech output.
    pub fn stop(&mut self) -> Result<()> {
        self.interrupt()
    }

    /// Pauses current speech playback.
    pub fn pause(&mut self) -> Result<()> {
        if let Some(driver) = self.active_driver_mut() {
            driver.pause()
        } else {
            Ok(())
        }
    }

    /// Resumes paused speech playback.
    pub fn resume(&mut self) -> Result<()> {
        if let Some(driver) = self.active_driver_mut() {
            driver.resume()
        } else {
            Ok(())
        }
    }

    /// Sets speaking rate (-10 to 10 or 0 to 100).
    pub fn set_rate(&mut self, rate: i32) -> Result<()> {
        if let Some(driver) = self.active_driver_mut() {
            driver.set_rate(rate)
        } else {
            Err(SpeechError::SynthNotAvailable("No active synthesizer".to_string()))
        }
    }

    /// Gets current speaking rate.
    pub fn get_rate(&self) -> i32 {
        self.active_driver().map(|d| d.get_rate()).unwrap_or(0)
    }

    /// Sets volume (0 to 100).
    pub fn set_volume(&mut self, volume: u16) -> Result<()> {
        if let Some(driver) = self.active_driver_mut() {
            driver.set_volume(volume)
        } else {
            Err(SpeechError::SynthNotAvailable("No active synthesizer".to_string()))
        }
    }

    /// Gets current volume (0 to 100).
    pub fn get_volume(&self) -> u16 {
        self.active_driver().map(|d| d.get_volume()).unwrap_or(100)
    }
}

impl Default for SpeechHub {
    fn default() -> Self {
        Self::default_platform().unwrap_or_else(|_| Self::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::MockSynthesizer;

    #[test]
    fn test_speech_hub_registration_and_switching() {
        let mut hub = SpeechHub::new();
        hub.register_driver(Box::new(MockSynthesizer::new()));

        assert_eq!(hub.active_synthesizer_name(), Some("mock"));
        assert_eq!(hub.available_synthesizers().len(), 1);

        let err = hub.set_active_synthesizer("non_existent");
        assert!(err.is_err());
    }

    #[test]
    fn test_speech_hub_speak_and_interrupt() {
        let mut hub = SpeechHub::new();
        let mock = MockSynthesizer::new();
        let mock_clone = mock.clone();
        hub.register_driver(Box::new(mock));

        // Speak utterance
        hub.speak("Hello from bit_sr screen reader", SpeechPriority::Now).unwrap();
        assert_eq!(mock_clone.get_spoken_history(), vec!["Hello from bit_sr screen reader".to_string()]);

        // Instant speech interrupt
        hub.interrupt().unwrap();

        // Volume and rate
        hub.set_volume(85).unwrap();
        assert_eq!(hub.get_volume(), 85);

        hub.set_rate(2).unwrap();
        assert_eq!(hub.get_rate(), 2);
    }
}
