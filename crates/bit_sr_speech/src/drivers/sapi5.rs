//! Windows Native SAPI 5 Synthesizer Driver (ISpVoice).
//! Uses Microsoft Speech API (sapi.dll) built into all Windows installations.
//! Architecture: Offloaded to a dedicated background worker thread with lock-free channel dispatch
//! to prevent COM audio pipeline stalls on the core screen reader loop.

use crate::error::{Result, SpeechError};
use crate::synthesizer::SynthesizerDriver;
use crate::voice::{VoiceGender, VoiceInfo};
use crossbeam_channel::{unbounded, Sender};
use std::thread::JoinHandle;
use windows::core::PCWSTR;
use windows::Win32::Media::Speech::{
    IEnumSpObjectTokens, ISpObjectTokenCategory, ISpVoice, SPF_ASYNC,
    SPF_PURGEBEFORESPEAK, SPCAT_VOICES, SpObjectTokenCategory, SpVoice,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};

/// RAII Guard ensuring COM apartment is initialized for the lifetime of the thread.
struct ComInitGuard {
    needs_uninit: bool,
}

impl ComInitGuard {
    fn new() -> Self {
        unsafe {
            // Initialize as MTA; returns S_OK or S_FALSE if already MTA.
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

/// Commands sent to the background SAPI 5 worker thread.
enum SapiWorkerCmd {
    Speak { text: String, interrupt: bool },
    Stop,
    Pause,
    Resume,
    SetRate(i32),
    SetVolume(u16),
    SetVoice(String),
}

/// Native SAPI 5 synthesizer driver for Windows.
/// Offloads all COM calls to a dedicated worker thread so that the caller
/// never experiences SAPI audio pipeline delays.
pub struct Sapi5Synthesizer {
    cmd_tx: Option<Sender<SapiWorkerCmd>>,
    worker_handle: Option<JoinHandle<()>>,
    voices: Vec<VoiceInfo>,
    current_voice: Option<VoiceInfo>,
    rate: i32,
    volume: u16,
}

// Thread-safe handle to SAPI 5 synthesizer worker
unsafe impl Send for Sapi5Synthesizer {}
unsafe impl Sync for Sapi5Synthesizer {}

impl Sapi5Synthesizer {
    /// Creates a new instance of the SAPI 5 speech engine with an asynchronous worker thread.
    pub fn new() -> Result<Self> {
        let (cmd_tx, cmd_rx) = unbounded::<SapiWorkerCmd>();
        let (init_tx, init_rx) = crossbeam_channel::bounded::<
            Result<(Vec<VoiceInfo>, Option<VoiceInfo>, i32, u16)>,
        >(1);

        let worker_handle = std::thread::Builder::new()
            .name("bit_sr_sapi5_worker".to_string())
            .spawn(move || {
                let com_guard = ComInitGuard::new();

                let voice: ISpVoice = match unsafe { CoCreateInstance(&SpVoice, None, CLSCTX_ALL) } {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = init_tx.send(Err(SpeechError::PlatformError(format!(
                            "Failed to create ISpVoice: {:?}",
                            e
                        ))));
                        return;
                    }
                };

                let mut rate = 0;
                let mut volume = 100;
                unsafe {
                    let _ = voice.GetRate(&mut rate);
                    let _ = voice.GetVolume(&mut volume);
                }

                let voices = unsafe { query_available_voices() };
                let current_voice = unsafe { query_current_voice(&voice) };

                // Signal successful initialization
                let _ = init_tx.send(Ok((voices, current_voice, rate, volume)));

                // Main speech worker loop
                while let Ok(cmd) = cmd_rx.recv() {
                    match cmd {
                        SapiWorkerCmd::Speak { text, interrupt } => {
                            let mut latest_text = text;
                            if interrupt {
                                // Coalesce pending speak requests to avoid speaking obsolete text
                                while let Ok(next) = cmd_rx.try_recv() {
                                    match next {
                                        SapiWorkerCmd::Speak {
                                            text: t,
                                            interrupt: true,
                                        } => {
                                            latest_text = t;
                                        }
                                        SapiWorkerCmd::Stop => {
                                            latest_text.clear();
                                            break;
                                        }
                                        other => {
                                            handle_aux_cmd(&voice, other);
                                        }
                                    }
                                }
                            }

                            if !latest_text.is_empty() {
                                let wide: Vec<u16> = latest_text
                                    .encode_utf16()
                                    .chain(std::iter::once(0))
                                    .collect();
                                let flags = if interrupt {
                                    (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0) as u32
                                } else {
                                    SPF_ASYNC.0 as u32
                                };
                                unsafe {
                                    let _ = voice.Speak(PCWSTR(wide.as_ptr()), flags, None);
                                }
                            }
                        }
                        SapiWorkerCmd::Stop => {
                            // Discard any pending speak commands
                            while let Ok(next) = cmd_rx.try_recv() {
                                if let SapiWorkerCmd::Speak { .. } = next {
                                    // discard
                                } else {
                                    handle_aux_cmd(&voice, next);
                                }
                            }
                            let flags = (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0) as u32;
                            unsafe {
                                let _ = voice.Speak(PCWSTR::null(), flags, None);
                            }
                        }
                        other => {
                            handle_aux_cmd(&voice, other);
                        }
                    }
                }

                // Clean up voice inside worker apartment before thread exit
                drop(voice);
                drop(com_guard);
            })
            .map_err(|e| SpeechError::PlatformError(format!("Failed to spawn SAPI worker: {:?}", e)))?;

        // Wait for worker initialization confirmation
        let (voices, current_voice, rate, volume) = init_rx
            .recv()
            .map_err(|_| SpeechError::PlatformError("SAPI worker thread died during init".to_string()))??;

        Ok(Self {
            cmd_tx: Some(cmd_tx),
            worker_handle: Some(worker_handle),
            voices,
            current_voice,
            rate,
            volume,
        })
    }
}

fn handle_aux_cmd(voice: &ISpVoice, cmd: SapiWorkerCmd) {
    match cmd {
        SapiWorkerCmd::Pause => unsafe {
            let _ = voice.Pause();
        },
        SapiWorkerCmd::Resume => unsafe {
            let _ = voice.Resume();
        },
        SapiWorkerCmd::SetRate(rate) => unsafe {
            let clamped = rate.clamp(-10, 10);
            let _ = voice.SetRate(clamped);
        },
        SapiWorkerCmd::SetVolume(volume) => unsafe {
            let clamped = volume.min(100);
            let _ = voice.SetVolume(clamped);
        },
        SapiWorkerCmd::SetVoice(voice_id) => unsafe {
            let _ = set_voice_token(voice, &voice_id);
        },
        SapiWorkerCmd::Speak { .. } | SapiWorkerCmd::Stop => {}
    }
}

/// Obtains an enumeration of all installed SAPI 5 voice tokens.
unsafe fn get_token_enum() -> Result<IEnumSpObjectTokens> {
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

unsafe fn query_available_voices() -> Vec<VoiceInfo> {
    unsafe {
        let mut voices = Vec::new();
        let enum_tokens = match get_token_enum() {
            Ok(e) => e,
            Err(_) => return voices,
        };

        let mut count = 0;
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
                    if s.is_empty() {
                        format!("Voice {}", i + 1)
                    } else {
                        s
                    }
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

        voices
    }
}

unsafe fn query_current_voice(voice: &ISpVoice) -> Option<VoiceInfo> {
    unsafe {
        if let Ok(token) = voice.GetVoice() {
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
                if s.is_empty() {
                    "Default Voice".to_string()
                } else {
                    s
                }
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

unsafe fn set_voice_token(voice: &ISpVoice, voice_id: &str) -> Result<()> {
    unsafe {
        let enum_tokens = get_token_enum()?;
        let mut count = 0;
        enum_tokens
            .GetCount(&mut count)
            .map_err(|e| SpeechError::PlatformError(format!("{:?}", e)))?;

        for i in 0..count {
            if let Ok(token) = enum_tokens.Item(i) {
                if let Ok(pwstr) = token.GetId() {
                    let s = pwstr.to_string().unwrap_or_default();
                    windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.as_ptr() as *const _));
                    if s == voice_id {
                        voice
                            .SetVoice(&token)
                            .map_err(|e| SpeechError::PlatformError(format!("SetVoice failed: {:?}", e)))?;
                        return Ok(());
                    }
                }
            }
        }

        Err(SpeechError::VoiceNotFound(voice_id.to_string()))
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
        self.voices.clone()
    }

    fn current_voice(&self) -> Option<VoiceInfo> {
        self.current_voice.clone()
    }

    fn set_voice(&mut self, voice_id: &str) -> Result<()> {
        if let Some(info) = self.voices.iter().find(|v| v.id == voice_id) {
            self.current_voice = Some(info.clone());
            if let Some(ref tx) = self.cmd_tx {
                let _ = tx.send(SapiWorkerCmd::SetVoice(voice_id.to_string()));
            }
            Ok(())
        } else {
            Err(SpeechError::VoiceNotFound(voice_id.to_string()))
        }
    }

    fn speak(&mut self, text: &str, interrupt: bool) -> Result<()> {
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(SapiWorkerCmd::Speak {
                text: text.to_string(),
                interrupt,
            });
            Ok(())
        } else {
            Err(SpeechError::SynthNotAvailable(
                "SAPI worker not running".to_string(),
            ))
        }
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(SapiWorkerCmd::Stop);
            Ok(())
        } else {
            Ok(())
        }
    }

    fn pause(&mut self) -> Result<()> {
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(SapiWorkerCmd::Pause);
            Ok(())
        } else {
            Ok(())
        }
    }

    fn resume(&mut self) -> Result<()> {
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(SapiWorkerCmd::Resume);
            Ok(())
        } else {
            Ok(())
        }
    }

    fn set_rate(&mut self, rate: i32) -> Result<()> {
        let clamped = rate.clamp(-10, 10);
        self.rate = clamped;
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(SapiWorkerCmd::SetRate(clamped));
        }
        Ok(())
    }

    fn get_rate(&self) -> i32 {
        self.rate
    }

    fn set_volume(&mut self, volume: u16) -> Result<()> {
        let clamped = volume.min(100);
        self.volume = clamped;
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(SapiWorkerCmd::SetVolume(clamped));
        }
        Ok(())
    }

    fn get_volume(&self) -> u16 {
        self.volume
    }
}

impl Drop for Sapi5Synthesizer {
    fn drop(&mut self) {
        // Disconnect channel to terminate worker loop
        drop(self.cmd_tx.take());
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}
