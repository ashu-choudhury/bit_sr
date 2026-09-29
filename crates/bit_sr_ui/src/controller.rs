//! UI Controller and Asynchronous Event Loop Coordinator.
//!
//! Manages Menu and Settings windows on a dedicated UI thread,
//! communicating with the screen reader engine coordinator via non-blocking channels.

use crate::menu::MenuAction;
use crate::settings::UiSettings;
use crate::SettingsWindow;
use crossbeam_channel::{bounded, Receiver, Sender};
use slint::ComponentHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

#[derive(Debug, Clone)]
pub enum UiCommand {
    OpenMenu,
    OpenSettings,
    CloseMenu,
    CloseAll,
}

#[derive(Debug, Clone)]
pub enum UiEvent {
    Action(MenuAction),
    SettingsSaved(UiSettings),
}

/// Handle held by the Engine Coordinator to communicate with the UI thread.
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

    /// Sends a command to open the left-side screen reader menu.
    pub fn open_menu(&self) {
        let _ = self.command_tx.try_send(UiCommand::OpenMenu);
    }

    /// Sends a command to open the Settings window.
    pub fn open_settings(&self) {
        let _ = self.command_tx.try_send(UiCommand::OpenSettings);
    }

    /// Receives asynchronous UI events emitted by the user (menu actions, settings changes).
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

    setup_settings_callbacks(&settings, evt_tx.clone());

    let settings_weak = settings.as_weak();
    let evt_tx_cmd = evt_tx;

    // Background thread that handles commands
    thread::Builder::new()
        .name("bit_sr_ui_bridge".to_string())
        .spawn(move || {
            while let Ok(cmd) = cmd_rx.recv() {
                let s_weak = settings_weak.clone();
                let evt_tx_action = evt_tx_cmd.clone();

                match cmd {
                    UiCommand::OpenMenu => {
                        #[cfg(windows)]
                        {
                            // Launch the authentic native Windows popup menu (identical to NVDA's PopupMenu)
                            if let Some(action) = crate::menu::show_native_popup_menu(24, -1) {
                                if action == MenuAction::OpenSettings
                                    || action == MenuAction::OpenSpeechSettings
                                    || action == MenuAction::OpenKeyboardSettings
                                    || action == MenuAction::OpenPluginManager
                                {
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(s) = s_weak.upgrade() {
                                            show_settings(&s);
                                        }
                                    });
                                } else {
                                    let _ = evt_tx_action.try_send(UiEvent::Action(action));
                                }
                            }
                        }
                    }
                    UiCommand::OpenSettings => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(s) = s_weak.upgrade() {
                                show_settings(&s);
                            }
                        });
                    }
                    UiCommand::CloseMenu => {
                        // Native Windows popup menus auto-close when dismissed or item selected
                    }
                    UiCommand::CloseAll => {
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
            restore_previous_focus();
        }
    });

    let settings_weak_cancel = settings.as_weak();
    settings.on_cancel_requested(move || {
        if let Some(s) = settings_weak_cancel.upgrade() {
            let _ = s.hide();
            restore_previous_focus();
        }
    });
}

// Global storage of previously focused HWND to restore focus cleanly upon closing
#[cfg(windows)]
static PREVIOUS_FOREGROUND_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

fn show_settings(settings: &SettingsWindow) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let current_hwnd = GetForegroundWindow();
        PREVIOUS_FOREGROUND_HWND.store(current_hwnd.0 as isize, Ordering::SeqCst);
    }

    if let Err(e) = settings.show() {
        log::error!("Failed to show SettingsWindow: {:?}", e);
    }

    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow};
        use windows::core::w;
        if let Ok(hwnd) = FindWindowW(None, w!("bit_sr Settings")) {
            if !hwnd.0.is_null() {
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }
}

fn restore_previous_focus() {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
        use windows::Win32::Foundation::HWND;

        let prev = PREVIOUS_FOREGROUND_HWND.swap(0, Ordering::SeqCst);
        if prev != 0 {
            let hwnd = HWND(prev as *mut _);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}
