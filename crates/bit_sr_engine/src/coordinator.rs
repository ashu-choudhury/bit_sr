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
    pub text_provider: Option<std::sync::Arc<dyn bit_sr_core::TextProvider>>,
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
            text_provider: None,
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
            text_provider: None,
        }
    }

    /// Sets the platform text provider for reading text at the caret.
    pub fn set_text_provider(&mut self, provider: std::sync::Arc<dyn bit_sr_core::TextProvider>) {
        self.text_provider = Some(provider);
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

                // Check registered screen reader commands
                if let Some(cmd) = self.command_dispatcher.process_key(&key) {
                    return self.execute_command(cmd);
                }

                // Caret Navigation in Edit Controls and Documents (Arrow keys)
                if key.modifiers.is_empty() || key.modifiers == bit_sr_core::input::KeyModifiers::CONTROL {
                    match key.key {
                        bit_sr_core::input::Key::UpArrow | bit_sr_core::input::Key::DownArrow => {
                            if let Some(ref provider) = self.text_provider {
                                std::thread::sleep(std::time::Duration::from_millis(15));
                                if let Some(text) = provider.get_text_at_caret(bit_sr_core::TextUnit::Line) {
                                    let announcement = if text.trim().is_empty() {
                                        self.loc.t("format.blank").to_string()
                                    } else {
                                        text.trim_end_matches(&['\r', '\n'][..]).to_string()
                                    };
                                    let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                                    return EngineAction::Spoke(announcement);
                                }
                            }
                        }
                        bit_sr_core::input::Key::LeftArrow | bit_sr_core::input::Key::RightArrow => {
                            if key.modifiers == bit_sr_core::input::KeyModifiers::CONTROL {
                                if let Some(ref provider) = self.text_provider {
                                    std::thread::sleep(std::time::Duration::from_millis(15));
                                    if let Some(text) = provider.get_text_at_caret(bit_sr_core::TextUnit::Word) {
                                        let announcement = if text.trim().is_empty() {
                                            self.loc.t("format.blank").to_string()
                                        } else {
                                            text.trim().to_string()
                                        };
                                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                                        return EngineAction::Spoke(announcement);
                                    }
                                }
                            } else {
                                if let Some(ref provider) = self.text_provider {
                                    std::thread::sleep(std::time::Duration::from_millis(15));
                                    if let Some(text) = provider.get_text_at_caret(bit_sr_core::TextUnit::Character) {
                                        let announcement = if text.is_empty() || text == "\r" || text == "\n" {
                                            self.loc.t("format.blank").to_string()
                                        } else if text == " " {
                                            self.loc.t("key.space").to_string()
                                        } else {
                                            text
                                        };
                                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                                        return EngineAction::Spoke(announcement);
                                    }
                                }
                            }
                        }
                        bit_sr_core::input::Key::PageUp | bit_sr_core::input::Key::PageDown => {
                            if let Some(ref provider) = self.text_provider {
                                std::thread::sleep(std::time::Duration::from_millis(20));
                                if let Some(text) = provider.get_text_at_caret(bit_sr_core::TextUnit::Line) {
                                    let announcement = if text.trim().is_empty() {
                                        self.loc.t("format.blank").to_string()
                                    } else {
                                        text.trim_end_matches(&['\r', '\n'][..]).to_string()
                                    };
                                    let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                                    return EngineAction::Spoke(announcement);
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Typing Echo: Printable characters, Space, Enter, Backspace, Delete
                if !key.modifiers.intersects(
                    bit_sr_core::input::KeyModifiers::CONTROL
                        | bit_sr_core::input::KeyModifiers::ALT
                        | bit_sr_core::input::KeyModifiers::SUPER
                        | bit_sr_core::input::KeyModifiers::SR,
                ) {
                    match key.key {
                        bit_sr_core::input::Key::Space => {
                            let msg = self.loc.t("key.space");
                            let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                            return EngineAction::Spoke(msg.to_string());
                        }
                        bit_sr_core::input::Key::Enter | bit_sr_core::input::Key::NumpadEnter => {
                            let msg = self.loc.t("key.enter");
                            let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                            return EngineAction::Spoke(msg.to_string());
                        }
                        bit_sr_core::input::Key::Backspace => {
                            let msg = self.loc.t("key.backspace");
                            let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                            return EngineAction::Spoke(msg.to_string());
                        }
                        bit_sr_core::input::Key::Delete => {
                            let msg = self.loc.t("key.delete");
                            let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                            return EngineAction::Spoke(msg.to_string());
                        }
                        _ => {
                            if let Some(ref text) = key.text {
                                let trimmed = text.trim();
                                if !trimmed.is_empty() {
                                    let _ = self.speech_hub.speak(trimmed, SpeechPriority::Now);
                                    return EngineAction::Spoke(trimmed.to_string());
                                }
                            }
                        }
                    }
                }

                EngineAction::None
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

    struct MockTextProvider {
        current_character: std::sync::Mutex<Option<String>>,
        current_word: std::sync::Mutex<Option<String>>,
        current_line: std::sync::Mutex<Option<String>>,
    }

    impl MockTextProvider {
        fn new(char_text: Option<&str>, word_text: Option<&str>, line_text: Option<&str>) -> Self {
            Self {
                current_character: std::sync::Mutex::new(char_text.map(|s| s.to_string())),
                current_word: std::sync::Mutex::new(word_text.map(|s| s.to_string())),
                current_line: std::sync::Mutex::new(line_text.map(|s| s.to_string())),
            }
        }
    }

    impl bit_sr_core::TextProvider for MockTextProvider {
        fn get_text_at_caret(&self, unit: bit_sr_core::TextUnit) -> Option<String> {
            match unit {
                bit_sr_core::TextUnit::Character => self.current_character.lock().unwrap().clone(),
                bit_sr_core::TextUnit::Word => self.current_word.lock().unwrap().clone(),
                bit_sr_core::TextUnit::Line | bit_sr_core::TextUnit::Paragraph | bit_sr_core::TextUnit::Document => {
                    self.current_line.lock().unwrap().clone()
                }
            }
        }
    }

    #[test]
    fn test_coordinator_typing_echo() {
        let mut hub = SpeechHub::new();
        let mock = MockSynthesizer::new();
        let history = mock.clone();
        hub.register_driver(Box::new(mock));

        let mut coordinator = EngineCoordinator::new(hub);

        // Echo typed character 'a'
        let mut key_a = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::A,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        key_a.text = Some("a".to_string());
        let action_a = coordinator.handle_event(AccessibilityEvent::Input(key_a));
        assert_eq!(action_a, EngineAction::Spoke("a".to_string()));

        // Space
        let key_space = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::Space,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action_space = coordinator.handle_event(AccessibilityEvent::Input(key_space));
        assert_eq!(action_space, EngineAction::Spoke("space".to_string()));

        // Enter
        let key_enter = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::Enter,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action_enter = coordinator.handle_event(AccessibilityEvent::Input(key_enter));
        assert_eq!(action_enter, EngineAction::Spoke("enter".to_string()));

        // Backspace
        let key_backspace = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::Backspace,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action_backspace = coordinator.handle_event(AccessibilityEvent::Input(key_backspace));
        assert_eq!(action_backspace, EngineAction::Spoke("backspace".to_string()));

        // Delete
        let key_delete = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::Delete,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action_delete = coordinator.handle_event(AccessibilityEvent::Input(key_delete));
        assert_eq!(action_delete, EngineAction::Spoke("delete".to_string()));

        assert_eq!(
            history.get_spoken_history(),
            vec!["a", "space", "enter", "backspace", "delete"]
        );
    }

    #[test]
    fn test_coordinator_caret_navigation() {
        let mut hub = SpeechHub::new();
        let mock = MockSynthesizer::new();
        let history = mock.clone();
        hub.register_driver(Box::new(mock));

        let mut coordinator = EngineCoordinator::new(hub);
        let provider = std::sync::Arc::new(MockTextProvider::new(
            Some("h"),
            Some("hello"),
            Some("hello world\r\n"),
        ));
        coordinator.set_text_provider(provider.clone());

        // DownArrow -> Line
        let key_down = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::DownArrow,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action_down = coordinator.handle_event(AccessibilityEvent::Input(key_down));
        assert_eq!(action_down, EngineAction::Spoke("hello world".to_string()));

        // RightArrow -> Character
        let key_right = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::RightArrow,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action_right = coordinator.handle_event(AccessibilityEvent::Input(key_right.clone()));
        assert_eq!(action_right, EngineAction::Spoke("h".to_string()));

        // Space character on RightArrow
        *provider.current_character.lock().unwrap() = Some(" ".to_string());
        let action_space_char = coordinator.handle_event(AccessibilityEvent::Input(key_right));
        assert_eq!(action_space_char, EngineAction::Spoke("space".to_string()));

        // Empty line -> blank
        *provider.current_line.lock().unwrap() = Some("".to_string());
        let key_up = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::UpArrow,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action_up = coordinator.handle_event(AccessibilityEvent::Input(key_up));
        assert_eq!(action_up, EngineAction::Spoke("blank".to_string()));

        // Ctrl + RightArrow -> Word
        let key_ctrl_right = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::RightArrow,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::CONTROL,
        );
        let action_word = coordinator.handle_event(AccessibilityEvent::Input(key_ctrl_right));
        assert_eq!(action_word, EngineAction::Spoke("hello".to_string()));

        assert_eq!(
            history.get_spoken_history(),
            vec!["hello world", "h", "space", "blank", "hello"]
        );
    }
}


