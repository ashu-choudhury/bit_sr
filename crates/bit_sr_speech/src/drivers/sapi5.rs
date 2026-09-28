//! Windows Native SAPI 5 Synthesizer Driver (ISpVoice).
//! Uses Microsoft Speech API (sapi.dll) built into all Windows installations.

use crate::error::{Result, SpeechError};
use crate::synthesizer::SynthesizerDriver;
use crate::voice::{VoiceGender, VoiceInfo};
use windows::core::PCWSTR;
use windows::Win32::Media::Speech::{
    IEnumSpObjectTokens, ISpObjectTokenCategory, ISpVoice, SPF_ASYNC,
    SPF_PURGEBEFORESPEAK, SPCAT_VOICES, SpObjectTokenCategory, SpVoice,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};

/// RAII Guard ensuring COM apartment is initialized for the lifetime of the synthesizer
struct ComInitGuard {
    needs_uninit: bool,
}

impl ComInitGuard {
    fn new() -> Self {
        unsafe {
            // Attempt to initialize as MTA; returns S_OK or S_FALSE if already MTA, or RPC_E_CHANGED_MODE if STA.
            let res = CoInitializeEx(None, COINIT_MULTITHREADED);
            let needs_uninit = res.is_ok();
            Self { needs_uninit }
        }
    }
}

impl Drop for ComInitGuard {
    fn drop(&mut self) {
        if self.needs_uninit {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

/// Native SAPI 5 synthesizer driver for Windows.
pub struct Sapi5Synthesizer {
    _com_guard: ComInitGuard,
    voice: ISpVoice,
    rate: i32,
    volume: u16,
}

// Windows COM MTA thread-safe wrapper
unsafe impl Send for Sapi5Synthesizer {}
unsafe impl Sync for Sapi5Synthesizer {}

impl Sapi5Synthesizer {
    /// Creates a new instance of the SAPI 5 speech engine.
    pub fn new() -> Result<Self> {
        let com_guard = ComInitGuard::new();

        let voice: ISpVoice = unsafe {
            CoCreateInstance(&SpVoice, None, CLSCTX_ALL)
                .map_err(|e| SpeechError::PlatformError(format!("Failed to create ISpVoice: {:?}", e)))?
        };

        let mut rate = 0;
        let mut volume = 100;
        unsafe {
            let _ = voice.GetRate(&mut rate);
            let _ = voice.GetVolume(&mut volume);
        }

        Ok(Self {
            _com_guard: com_guard,
            voice,
            rate,
            volume,
        })
    }

    /// Obtains an enumeration of all installed SAPI 5 voice tokens.
    fn get_token_enum(&self) -> Result<IEnumSpObjectTokens> {
        unsafe {
            let category: ISpObjectTokenCategory = CoCreateInstance(&SpObjectTokenCategory, None, CLSCTX_ALL)
                .map_err(|e| SpeechError::PlatformError(format!("Failed to create SpObjectTokenCategory: {:?}", e)))?;

            category
                .SetId(SPCAT_VOICES, false)
                .map_err(|e| SpeechError::PlatformError(format!("Failed to SetId(SPCAT_VOICES): {:?}", e)))?;

            category
                .EnumTokens(PCWSTR::null(), PCWSTR::null())
                .map_err(|e| SpeechError::PlatformError(format!("Failed to EnumTokens: {:?}", e)))
        }
    }
}

impl SynthesizerDriver for Sapi5Synthesizer {
    fn name(&self) -> &str {
        "sapi5"
    }

    fn display_name(&self) -> &str {
        "Microsoft Speech API (SAPI 5)"
    }

    fn available_voices(&self) -> Vec<VoiceInfo> {
        let mut voices = Vec::new();
        let enum_tokens = match self.get_token_enum() {
            Ok(e) => e,
            Err(_) => return voices,
        };

        let mut count = 0;
        unsafe {
            if enum_tokens.GetCount(&mut count).is_err() {
                return voices;
            }

            for i in 0..count {
                if let Ok(token) = enum_tokens.Item(i) {
                    let id = if let Ok(pwstr) = token.GetId() {
                        let s = pwstr.to_string().unwrap_or_default();
                        windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.as_ptr() as *const _));
                        s
                    } else {
                        format!("sapi5:voice_{}", i)
                    };

                    let name = if let Ok(pwstr) = token.GetStringValue(PCWSTR::null()) {
                        let s = pwstr.to_string().unwrap_or_default();
                        windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.as_ptr() as *const _));
                        if s.is_empty() { format!("Voice {}", i + 1) } else { s }
                    } else {
                        format!("Voice {}", i + 1)
                    };

                    voices.push(VoiceInfo {
                        id,
                        name,
                        language: "unknown".to_string(),
                        gender: Some(VoiceGender::Neutral),
                    });
                }
            }
        }

        voices
    }

    fn current_voice(&self) -> Option<VoiceInfo> {
        unsafe {
            if let Ok(token) = self.voice.GetVoice() {
                let id = if let Ok(pwstr) = token.GetId() {
                    let s = pwstr.to_string().unwrap_or_default();
                    windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.as_ptr() as *const _));
                    s
                } else {
                    "sapi5:default".to_string()
                };

                let name = if let Ok(pwstr) = token.GetStringValue(PCWSTR::null()) {
                    let s = pwstr.to_string().unwrap_or_default();
                    windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.as_ptr() as *const _));
                    if s.is_empty() { "Default Voice".to_string() } else { s }
                } else {
                    "Default Voice".to_string()
                };

                Some(VoiceInfo {
                    id,
                    name,
                    language: "unknown".to_string(),
                    gender: Some(VoiceGender::Neutral),
                })
            } else {
                None
            }
        }
    }

    fn set_voice(&mut self, voice_id: &str) -> Result<()> {
        let enum_tokens = self.get_token_enum()?;
        let mut count = 0;
        unsafe {
            enum_tokens
                .GetCount(&mut count)
                .map_err(|e| SpeechError::PlatformError(format!("{:?}", e)))?;

            for i in 0..count {
                if let Ok(token) = enum_tokens.Item(i) {
                    if let Ok(pwstr) = token.GetId() {
                        let s = pwstr.to_string().unwrap_or_default();
                        windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.as_ptr() as *const _));
                        if s == voice_id {
                            self.voice
                                .SetVoice(&token)
                                .map_err(|e| SpeechError::PlatformError(format!("SetVoice failed: {:?}", e)))?;
                            return Ok(());
                        }
                    }
                }
            }
        }

        Err(SpeechError::VoiceNotFound(voice_id.to_string()))
    }

    fn speak(&mut self, text: &str, interrupt: bool) -> Result<()> {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let flags = if interrupt {
            (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0) as u32
        } else {
            SPF_ASYNC.0 as u32
        };

        unsafe {
            self.voice
                .Speak(PCWSTR(wide.as_ptr()), flags, None)
                .map_err(|e| SpeechError::SynthesisFailed(format!("{:?}", e)))?;
        }

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        let flags = (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0) as u32;
        unsafe {
            self.voice
                .Speak(PCWSTR::null(), flags, None)
                .map_err(|e| SpeechError::SynthesisFailed(format!("{:?}", e)))?;
        }
        Ok(())
    }

    fn pause(&mut self) -> Result<()> {
        unsafe {
            self.voice
                .Pause()
                .map_err(|e| SpeechError::PlatformError(format!("{:?}", e)))?;
        }
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        unsafe {
            self.voice
                .Resume()
                .map_err(|e| SpeechError::PlatformError(format!("{:?}", e)))?;
        }
        Ok(())
    }

    fn set_rate(&mut self, rate: i32) -> Result<()> {
        // SAPI 5 rate range is [-10, 10]
        let clamped = rate.clamp(-10, 10);
        unsafe {
            self.voice
                .SetRate(clamped)
                .map_err(|e| SpeechError::PlatformError(format!("{:?}", e)))?;
        }
        self.rate = clamped;
        Ok(())
    }

    fn get_rate(&self) -> i32 {
        self.rate
    }

    fn set_volume(&mut self, volume: u16) -> Result<()> {
        let clamped = volume.min(100);
        unsafe {
            self.voice
                .SetVolume(clamped)
                .map_err(|e| SpeechError::PlatformError(format!("{:?}", e)))?;
        }
        self.volume = clamped;
        Ok(())
    }

    fn get_volume(&self) -> u16 {
        self.volume
    }
}
