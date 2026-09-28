//! Engine Coordinator: Central orchestration connecting platform events, speech, and focus tracking.

use crate::commands::{CommandDispatcher, ScreenReaderCommand, SpeechMode};
use crate::formatter::{FormatterContext, SpeechFormatter};
use crate::tracker::{FocusTracker, FocusTransition};
use bit_sr_core::events::AccessibilityEvent;
use bit_sr_speech::{SpeechHub, SpeechPriority};
use crossbeam_channel::{select, Receiver};

#[derive(Debug, Clone, PartialEq)]
pub enum EngineAction {
    None,
    Spoke(String),
    Interrupted,
    Quit,
}

pub struct EngineCoordinator {
    pub speech_hub: SpeechHub,
    pub focus_tracker: FocusTracker,
    pub command_dispatcher: CommandDispatcher,
    pub formatter_context: FormatterContext,
    pub loc: std::sync::Arc<bit_sr_core::LocalizationManager>,
}

impl EngineCoordinator {
    pub fn new(speech_hub: SpeechHub) -> Self {
        Self::with_locale(speech_hub, "en")
    }

    pub fn with_locale(speech_hub: SpeechHub, locale: &str) -> Self {
        Self {
            speech_hub,
            focus_tracker: FocusTracker::new(),
            command_dispatcher: CommandDispatcher::new(),
            formatter_context: FormatterContext::default(),
            loc: std::sync::Arc::new(bit_sr_core::LocalizationManager::new(locale)),
        }
    }

    pub fn with_localization(
        speech_hub: SpeechHub,
        loc: std::sync::Arc<bit_sr_core::LocalizationManager>,
    ) -> Self {
        Self {
            speech_hub,
            focus_tracker: FocusTracker::new(),
            command_dispatcher: CommandDispatcher::new(),
            formatter_context: FormatterContext::default(),
            loc,
        }
    }

    /// Sets the active locale at runtime.
    pub fn set_locale(&self, locale: &str) {
        self.loc.set_locale(locale);
    }

    /// Handles a single incoming event from the platform.
    pub fn handle_event(&mut self, event: AccessibilityEvent) -> EngineAction {
        match event {
            AccessibilityEvent::SpeechInterrupt => {
                let _ = self.speech_hub.interrupt();
                EngineAction::Interrupted
            }

            AccessibilityEvent::CapsLockToggled(is_on) => {
                let msg = if is_on {
                    self.loc.t("system.capslock_on")
                } else {
                    self.loc.t("system.capslock_off")
                };
                let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                EngineAction::Spoke(msg.to_string())
            }

            AccessibilityEvent::Input(key) => {
                if key.action != bit_sr_core::input::KeyAction::Down {
                    return EngineAction::None;
                }

                // If in input help mode, describe keystroke without executing command
                if self.command_dispatcher.input_help_active {
                    let gesture = bit_sr_core::input::InputGesture::new(key.modifiers, key.key);
                    if let Some(cmd) = self.command_dispatcher.gesture_map.lookup(&gesture) {
                        if cmd == ScreenReaderCommand::ToggleInputHelp {
                            self.command_dispatcher.input_help_active = false;
                            #[cfg(windows)]
                            bit_sr_platform_windows::set_input_help_active(false);
                            let msg = self.loc.t("system.input_help_off");
                            let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                            return EngineAction::Spoke(msg.to_string());
                        }
                        let announcement = format!("{}: {}", gesture.display_name(), cmd.display_name_localized(&self.loc));
                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                        return EngineAction::Spoke(announcement);
                    } else {
                        let announcement = gesture.display_name();
                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                        return EngineAction::Spoke(announcement);
                    }
                }

                if let Some(cmd) = self.command_dispatcher.process_key(&key) {
                    self.execute_command(cmd)
                } else {
                    EngineAction::None
                }
            }

            AccessibilityEvent::Focus(node) => {
                if self.command_dispatcher.speech_mode == SpeechMode::Mute {
                    return EngineAction::None;
                }

                let (transition, focused_node) = self.focus_tracker.on_focus(node);
                if transition == FocusTransition::Redundant {
                    return EngineAction::None;
                }

                let mut announcement = String::new();

                if let FocusTransition::NewWindow { window_title } = transition {
                    if let Some(title) = window_title {
                        let win_prefix = self.loc.t_args("format.window_suffix", &[("title", &title)]);
                        announcement.push_str(&format!("{}, ", win_prefix));
                    }
                }

                let node_text = SpeechFormatter::format_focus_localized(
                    focused_node,
                    &mut self.formatter_context,
                    &self.loc,
                );
                announcement.push_str(&node_text);

                if !announcement.trim().is_empty() {
                    let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                    EngineAction::Spoke(announcement)
                } else {
                    EngineAction::None
                }
            }

            AccessibilityEvent::WindowActivated(node) => {
                let title = self.focus_tracker.on_window_activated(&node);
                if let Some(t) = title {
                    let announcement = self.loc.t_args("format.window_suffix", &[("title", &t)]);
                    let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                    EngineAction::Spoke(announcement)
                } else {
                    EngineAction::None
                }
            }

            AccessibilityEvent::StateChange { node, state, is_set } => {
                // Only speak state changes for the current focused node
                if let Some(focused) = self.focus_tracker.current_focus() {
                    if focused.id == node.id {
                        if let Some(state_str) = SpeechFormatter::format_state_change_localized(
                            state,
                            is_set,
                            &self.loc,
                        ) {
                            let _ = self.speech_hub.speak(state_str, SpeechPriority::Now);
                            return EngineAction::Spoke(state_str.to_string());
                        }
                    }
                }
                EngineAction::None
            }

            AccessibilityEvent::NameChange { node, new_name } => {
                if let Some(focused) = self.focus_tracker.current_focus() {
                    if focused.id == node.id {
                        if let Some(name) = new_name {
                            let _ = self.speech_hub.speak(&name, SpeechPriority::Now);
                            return EngineAction::Spoke(name);
                        }
                    }
                }
                EngineAction::None
            }

            _ => EngineAction::None,
        }
    }

    /// Executes a screen reader shortcut command.
    fn execute_command(&mut self, cmd: ScreenReaderCommand) -> EngineAction {
        match cmd {
            ScreenReaderCommand::Quit => {
                let msg = self.loc.t("system.app_exit");
                let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                EngineAction::Quit
            }

            ScreenReaderCommand::AnnounceTitle => {
                #[cfg(windows)]
                let live_title = bit_sr_platform_windows::get_foreground_window_title();
                #[cfg(not(windows))]
                let live_title: Option<String> = None;

                if let Some(ref lt) = live_title {
                    self.focus_tracker.set_window_title(lt.clone());
                }

                let title = live_title
                    .as_deref()
                    .or_else(|| self.focus_tracker.current_window_title())
                    .unwrap_or("Unknown window");
                let msg = self.loc.t_args("format.window_suffix", &[("title", title)]);
                let _ = self.speech_hub.speak(&msg, SpeechPriority::Now);
                EngineAction::Spoke(msg)
            }

            ScreenReaderCommand::RepeatFocus => {
                if let Some(focused) = self.focus_tracker.current_focus() {
                    let text = SpeechFormatter::format_focus_localized(
                        focused,
                        &mut self.formatter_context,
                        &self.loc,
                    );
                    let _ = self.speech_hub.speak(&text, SpeechPriority::Now);
                    EngineAction::Spoke(text)
                } else {
                    let msg = self.loc.t("system.no_focus");
                    let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                    EngineAction::Spoke(msg.to_string())
                }
            }

            ScreenReaderCommand::ToggleSpeechMode => {
                let msg = match self.command_dispatcher.speech_mode {
                    SpeechMode::Talk => self.loc.t("system.speech_talk"),
                    SpeechMode::Mute => self.loc.t("system.speech_mute"),
                };
                let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                EngineAction::Spoke(msg.to_string())
            }

            ScreenReaderCommand::VolumeUp => {
                let vol = (self.speech_hub.get_volume() + 10).min(100);
                let _ = self.speech_hub.set_volume(vol);
                let vol_str = vol.to_string();
                let msg = self.loc.t_args("system.volume", &[("vol", &vol_str)]);
                let _ = self.speech_hub.speak(&msg, SpeechPriority::Now);
                EngineAction::Spoke(msg)
            }

            ScreenReaderCommand::VolumeDown => {
                let vol = self.speech_hub.get_volume().saturating_sub(10);
                let _ = self.speech_hub.set_volume(vol);
                let vol_str = vol.to_string();
                let msg = self.loc.t_args("system.volume", &[("vol", &vol_str)]);
                let _ = self.speech_hub.speak(&msg, SpeechPriority::Now);
                EngineAction::Spoke(msg)
            }

            ScreenReaderCommand::RateFaster => {
                let rate = (self.speech_hub.get_rate() + 1).min(10);
                let _ = self.speech_hub.set_rate(rate);
                let rate_str = rate.to_string();
                let msg = self.loc.t_args("system.rate", &[("rate", &rate_str)]);
                let _ = self.speech_hub.speak(&msg, SpeechPriority::Now);
                EngineAction::Spoke(msg)
            }

            ScreenReaderCommand::RateSlower => {
                let rate = (self.speech_hub.get_rate() - 1).max(-10);
                let _ = self.speech_hub.set_rate(rate);
                let rate_str = rate.to_string();
                let msg = self.loc.t_args("system.rate", &[("rate", &rate_str)]);
                let _ = self.speech_hub.speak(&msg, SpeechPriority::Now);
                EngineAction::Spoke(msg)
            }

            ScreenReaderCommand::ToggleInputHelp => {
                #[cfg(windows)]
                bit_sr_platform_windows::set_input_help_active(self.command_dispatcher.input_help_active);

                let msg = if self.command_dispatcher.input_help_active {
                    self.loc.t("system.input_help_on")
                } else {
                    self.loc.t("system.input_help_off")
                };
                let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                EngineAction::Spoke(msg.to_string())
            }
        }
    }

    /// Runs the reactive event loop until shutdown is signaled or a Quit command is received.
    pub fn run(mut self, event_rx: Receiver<AccessibilityEvent>, shutdown_rx: Receiver<()>) {
        log::info!("Engine coordinator loop active and awaiting events...");
        loop {
            select! {
                recv(event_rx) -> msg => {
                    match msg {
                        Ok(event) => {
                            if self.handle_event(event) == EngineAction::Quit {
                                log::info!("Quit requested via screen reader hotkey.");
                                break;
                            }
                        }
                        Err(_) => {
                            log::info!("Event sender disconnected; exiting engine loop.");
                            break;
                        }
                    }
                }
                recv(shutdown_rx) -> _ => {
                    log::info!("Shutdown signal received; exiting engine loop.");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit_sr_core::node::NodeId;
    use bit_sr_core::roles::Role;
    use bit_sr_core::states::State;
    use bit_sr_speech::drivers::MockSynthesizer;

    #[test]
    fn test_coordinator_focus_and_repeat() {
        let mut hub = SpeechHub::new();
        let mock = MockSynthesizer::new();
        let history = mock.clone();
        hub.register_driver(Box::new(mock));

        let mut coordinator = EngineCoordinator::new(hub);

        let button = bit_sr_core::node::AccessibleNode {
            id: NodeId(1),
            name: Some("OK".to_string()),
            role: Role::Button,
            states: State::FOCUSABLE | State::FOCUSED,
            ..Default::default()
        };

        // Send Focus event
        let action = coordinator.handle_event(AccessibilityEvent::Focus(button));
        assert_eq!(action, EngineAction::Spoke("OK, button".to_string()));
        assert_eq!(history.get_spoken_history(), vec!["OK, button".to_string()]);

        // Send RepeatFocus command via keypress (Insert + Tab)
        let key_repeat = bit_sr_core::input::KeyEvent {
            key: bit_sr_core::input::Key::Tab,
            vk_code: 0x09,
            scan_code: 0,
            is_extended: false,
            is_injected: false,
            action: bit_sr_core::input::KeyAction::Down,
            modifiers: bit_sr_core::input::KeyModifiers::INSERT,
            text: None,
        };
        let action_repeat = coordinator.handle_event(AccessibilityEvent::Input(key_repeat));
        assert_eq!(action_repeat, EngineAction::Spoke("OK, button".to_string()));
    }

    #[test]
    fn test_coordinator_input_help_mode() {
        let mut hub = SpeechHub::new();
        let mock = MockSynthesizer::new();
        let _history = mock.clone();
        hub.register_driver(Box::new(mock));

        let mut coordinator = EngineCoordinator::new(hub);

        // Press SR + 1 to turn on input help
        let key_help = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::Num1,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::SR,
        );
        let action = coordinator.handle_event(AccessibilityEvent::Input(key_help.clone()));
        assert_eq!(action, EngineAction::Spoke("Input help on".to_string()));
        assert!(coordinator.command_dispatcher.input_help_active);

        // Press SR + T while in input help
        let key_t = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::T,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::SR,
        );
        let action_t = coordinator.handle_event(AccessibilityEvent::Input(key_t));
        assert_eq!(
            action_t,
            EngineAction::Spoke("SR + T: Announce Window Title".to_string())
        );

        // Press SR + 1 again to exit input help
        let action_off = coordinator.handle_event(AccessibilityEvent::Input(key_help));
        assert_eq!(action_off, EngineAction::Spoke("Input help off".to_string()));
        assert!(!coordinator.command_dispatcher.input_help_active);
    }

    #[test]
    fn test_coordinator_localized_spanish() {
        let mut hub = SpeechHub::new();
        let mock = MockSynthesizer::new();
        let _history = mock.clone();
        hub.register_driver(Box::new(mock));

        let mut coordinator = EngineCoordinator::with_locale(hub, "es");

        // CapsLock toggle in Spanish
        let action = coordinator.handle_event(AccessibilityEvent::CapsLockToggled(true));
        assert_eq!(action, EngineAction::Spoke("Bloqueo de mayúsculas activado".to_string()));

        // Speech mode toggle in Spanish
        let action_mute = coordinator.execute_command(ScreenReaderCommand::ToggleSpeechMode);
        assert_eq!(action_mute, EngineAction::Spoke("Voz activada".to_string()));

        // Focus in Spanish
        let button = bit_sr_core::node::AccessibleNode {
            id: NodeId(2),
            name: Some("Enviar".to_string()),
            role: Role::Button,
            states: State::FOCUSABLE | State::FOCUSED,
            ..Default::default()
        };
        let action_focus = coordinator.handle_event(AccessibilityEvent::Focus(button));
        assert_eq!(action_focus, EngineAction::Spoke("Enviar, botón".to_string()));
    }

    #[test]
    fn test_coordinator_localized_hindi() {
        let mut hub = SpeechHub::new();
        let mock = MockSynthesizer::new();
        hub.register_driver(Box::new(mock));

        let mut coordinator = EngineCoordinator::with_locale(hub, "hi");

        // CapsLock toggle in Hindi
        let action = coordinator.handle_event(AccessibilityEvent::CapsLockToggled(false));
        assert_eq!(action, EngineAction::Spoke("कैप्स लॉक बंद".to_string()));

        // Focus in Hindi
        let checkbox = bit_sr_core::node::AccessibleNode {
            id: NodeId(3),
            name: Some("स्वीकार करें".to_string()),
            role: Role::CheckBox,
            states: State::CHECKABLE | State::CHECKED,
            ..Default::default()
        };
        let action_focus = coordinator.handle_event(AccessibilityEvent::Focus(checkbox));
        assert_eq!(action_focus, EngineAction::Spoke("स्वीकार करें, चेक बॉक्स, चेक किया गया".to_string()));
    }
}

