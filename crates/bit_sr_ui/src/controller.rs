//! UI Controller and Asynchronous Event Loop Coordinator.
//!
//! Manages the Slint Settings Window on a dedicated UI thread,
//! communicating with the screen reader engine coordinator via non-blocking channels.
//! Completely platform-agnostic and free of OS-specific window handles.

use crate::settings::UiSettings;
use crate::SettingsWindow;
use crossbeam_channel::{bounded, Receiver, Sender};
use slint::ComponentHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

#[derive(Debug, Clone)]
pub enum UiCommand {
    OpenSettings,
    CloseSettings,
    CloseAll,
}

#[derive(Debug, Clone)]
pub enum UiEvent {
    SettingsSaved(UiSettings),
}

/// Handle held by the Engine Coordinator to communicate with the Slint UI thread.
#[derive(Clone)]
pub struct UiHandle {
    command_tx: Sender<UiCommand>,
    event_rx: Receiver<UiEvent>,
    is_active: Arc<AtomicBool>,
}

impl UiHandle {
    /// Launches the UI subsystem on a dedicated OS thread.
    pub fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
        let (cmd_tx, cmd_rx) = bounded::<UiCommand>(32);
        let (evt_tx, evt_rx) = bounded::<UiEvent>(32);
        let is_active = Arc::new(AtomicBool::new(true));
        let is_active_clone = is_active.clone();

        thread::Builder::new()
            .name("bit_sr_ui_thread".to_string())
            .spawn(move || {
                run_ui_loop(cmd_rx, evt_tx, is_active_clone);
            })?;

        Ok(Self {
            command_tx: cmd_tx,
            event_rx: evt_rx,
            is_active,
        })
    }

    /// Sends a command to open the Settings window.
    pub fn open_settings(&self) {
        let _ = self.command_tx.try_send(UiCommand::OpenSettings);
    }

    /// Sends a command to close the Settings window.
    pub fn close_settings(&self) {
        let _ = self.command_tx.try_send(UiCommand::CloseSettings);
    }

    /// Sends a command to close all GUI windows.
    pub fn close_all(&self) {
        let _ = self.command_tx.try_send(UiCommand::CloseAll);
    }

    /// Receives asynchronous UI events emitted by the user (settings changes).
    pub fn try_recv_event(&self) -> Option<UiEvent> {
        self.event_rx.try_recv().ok()
    }

    pub fn event_receiver(&self) -> &Receiver<UiEvent> {
        &self.event_rx
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }
}

fn run_ui_loop(
    cmd_rx: Receiver<UiCommand>,
    evt_tx: Sender<UiEvent>,
    is_active: Arc<AtomicBool>,
) {
    let settings = match SettingsWindow::new() {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to create Slint SettingsWindow: {:?}", e);
            is_active.store(false, Ordering::Relaxed);
            return;
        }
    };

    setup_settings_callbacks(&settings, evt_tx);

    let settings_weak = settings.as_weak();

    // Background bridge thread that processes commands from the engine coordinator
    thread::Builder::new()
        .name("bit_sr_ui_bridge".to_string())
        .spawn(move || {
            while let Ok(cmd) = cmd_rx.recv() {
                let s_weak = settings_weak.clone();

                match cmd {
                    UiCommand::OpenSettings => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(s) = s_weak.upgrade() {
                                if let Err(e) = s.show() {
                                    log::error!("Failed to show SettingsWindow: {:?}", e);
                                }
                            }
                        });
                    }
                    UiCommand::CloseSettings | UiCommand::CloseAll => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(s) = s_weak.upgrade() {
                                let _ = s.hide();
                            }
                        });
                    }
                }
            }
        })
        .ok();

    // Start Slint GUI event loop for settings window
    if let Err(e) = slint::run_event_loop() {
        log::error!("Slint event loop error: {:?}", e);
    }

    is_active.store(false, Ordering::Relaxed);
}

fn setup_settings_callbacks(settings: &SettingsWindow, evt_tx: Sender<UiEvent>) {
    let settings_weak = settings.as_weak();
    let evt_tx_save = evt_tx;

    settings.on_save_requested(move || {
        if let Some(s) = settings_weak.upgrade() {
            let ui_settings = UiSettings {
                locale_index: s.get_selected_language_index(),
                sr_modifier_index: s.get_sr_modifier_index(),
                start_at_logon: s.get_start_at_logon(),
                synth_driver_index: s.get_selected_synth_index(),
                voice_index: s.get_selected_voice_index(),
                speech_rate: s.get_speech_rate(),
                speech_volume: s.get_speech_volume(),
                speech_pitch: s.get_speech_pitch(),
                echo_mode_index: s.get_selected_echo_index(),
            };
            let _ = evt_tx_save.try_send(UiEvent::SettingsSaved(ui_settings));
            let _ = s.hide();
        }
    });

    let settings_weak_cancel = settings.as_weak();
    settings.on_cancel_requested(move || {
        if let Some(s) = settings_weak_cancel.upgrade() {
            let _ = s.hide();
        }
    });
}
