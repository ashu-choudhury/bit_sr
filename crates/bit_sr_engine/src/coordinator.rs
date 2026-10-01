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
    pub last_selection: Option<String>,
    pub ui_handle: Option<bit_sr_ui::UiHandle>,
    pub event_tx: Option<crossbeam_channel::Sender<AccessibilityEvent>>,
    pub web_controller: Option<bit_sr_web::WebController>,
    pub tree_provider: Option<std::sync::Arc<dyn bit_sr_core::tree::TreeProvider>>,
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
            last_selection: None,
            ui_handle: None,
            event_tx: None,
            web_controller: None,
            tree_provider: None,
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
            last_selection: None,
            ui_handle: None,
            event_tx: None,
            web_controller: None,
            tree_provider: None,
        }
    }

    /// Sets the UI handle for coordinating GUI windows and menus.
    pub fn set_ui_handle(&mut self, ui_handle: bit_sr_ui::UiHandle) {
        self.ui_handle = Some(ui_handle);
    }

    /// Sets the event sender for dispatching asynchronous platform events.
    pub fn set_event_tx(&mut self, tx: crossbeam_channel::Sender<AccessibilityEvent>) {
        self.event_tx = Some(tx);
    }

    /// Sets the platform text provider for reading text at the caret.
    pub fn set_text_provider(&mut self, provider: std::sync::Arc<dyn bit_sr_core::TextProvider>) {
        self.text_provider = Some(provider);
    }

    /// Sets the platform tree provider for harvesting accessibility trees in web documents.
    pub fn set_tree_provider(&mut self, provider: std::sync::Arc<dyn bit_sr_core::tree::TreeProvider>) {
        self.tree_provider = Some(provider);
    }

    /// Sets the active locale at runtime.
    pub fn set_locale(&self, locale: &str) {
        self.loc.set_locale(locale);
    }

    /// Sets the active web controller for web documents and webviews.
    pub fn set_web_controller(&mut self, controller: bit_sr_web::WebController) {
        self.web_controller = Some(controller);
    }

    /// Sets the active virtual buffer for web documents.
    pub fn set_web_buffer(&mut self, buffer: bit_sr_web::VirtualBuffer) {
        self.web_controller = Some(bit_sr_web::WebController::new(buffer));
    }

    /// Manually toggles Browse Mode / Focus Mode in the active WebController.
    pub fn toggle_browse_mode(&mut self) -> EngineAction {
        self.execute_command(ScreenReaderCommand::ToggleBrowseMode)
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

            AccessibilityEvent::MenuAction(action) => {
                match action {
                    bit_sr_core::menu::MenuAction::OpenSettings
                    | bit_sr_core::menu::MenuAction::OpenSpeechSettings
                    | bit_sr_core::menu::MenuAction::OpenKeyboardSettings
                    | bit_sr_core::menu::MenuAction::OpenPluginManager => {
                        if let Some(ref ui) = self.ui_handle {
                            ui.open_settings();
                        }
                        EngineAction::None
                    }
                    bit_sr_core::menu::MenuAction::SetSpeechModeTalk => {
                        self.command_dispatcher.speech_mode = SpeechMode::Talk;
                        let msg = self.loc.t("system.speech_talk");
                        let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                        EngineAction::Spoke(msg.to_string())
                    }
                    bit_sr_core::menu::MenuAction::SetSpeechModeMute => {
                        self.command_dispatcher.speech_mode = SpeechMode::Mute;
                        let msg = self.loc.t("system.speech_mute");
                        let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                        EngineAction::Spoke(msg.to_string())
                    }
                    bit_sr_core::menu::MenuAction::ToggleInputHelp => {
                        self.execute_command(ScreenReaderCommand::ToggleInputHelp)
                    }
                    bit_sr_core::menu::MenuAction::Quit => {
                        self.execute_command(ScreenReaderCommand::Quit)
                    }
                    _ => EngineAction::None,
                }
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

                // Select All (Ctrl + A)
                if key.modifiers == bit_sr_core::input::KeyModifiers::CONTROL
                    && key.key == bit_sr_core::input::Key::A
                {
                    let action = self.handle_selection_change(true);
                    if action != EngineAction::None {
                        return action;
                    }
                }

                // Selection Navigation (Shift + Navigation keys, with or without Ctrl)
                if key.modifiers.contains(bit_sr_core::input::KeyModifiers::SHIFT) {
                    match key.key {
                        bit_sr_core::input::Key::UpArrow
                        | bit_sr_core::input::Key::DownArrow
                        | bit_sr_core::input::Key::LeftArrow
                        | bit_sr_core::input::Key::RightArrow
                        | bit_sr_core::input::Key::Home
                        | bit_sr_core::input::Key::End
                        | bit_sr_core::input::Key::PageUp
                        | bit_sr_core::input::Key::PageDown => {
                            let action = self.handle_selection_change(false);
                            if action != EngineAction::None {
                                return action;
                            }
                        }
                        _ => {}
                    }
                }

                // Web Virtual Buffer Navigation (Browse Mode & Focus Mode)
                if let Some(ref mut wc) = self.web_controller {
                    let actions = wc.handle_key(&key);
                    for action in actions {
                        match action {
                            bit_sr_web::WebAction::Speak(text) => {
                                let _ = self.speech_hub.speak(&text, SpeechPriority::Now);
                                return EngineAction::Spoke(text);
                            }
                            bit_sr_web::WebAction::SwitchMode(mode) => {
                                let msg = match mode {
                                    bit_sr_web::NavigationMode::Browse => self.loc.t("web.browse_mode"),
                                    bit_sr_web::NavigationMode::Focus => self.loc.t("web.focus_mode"),
                                };
                                let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                                return EngineAction::Spoke(msg.to_string());
                            }
                            bit_sr_web::WebAction::PerformAction { .. } => {}
                            bit_sr_web::WebAction::PlaySound(_) | bit_sr_web::WebAction::None => {}
                            bit_sr_web::WebAction::PassThrough => break,
                        }
                    }
                    if wc.mode() == bit_sr_web::NavigationMode::Browse {
                        // In Browse Mode, intercept navigation keys and quick-nav without falling through
                        match key.key {
                            bit_sr_core::input::Key::UpArrow
                            | bit_sr_core::input::Key::DownArrow
                            | bit_sr_core::input::Key::LeftArrow
                            | bit_sr_core::input::Key::RightArrow
                            | bit_sr_core::input::Key::Home
                            | bit_sr_core::input::Key::End => return EngineAction::None,
                            _ => {
                                if bit_sr_web::QuickNavKey::from_key(
                                    key.key,
                                    key.modifiers.contains(bit_sr_core::input::KeyModifiers::SHIFT),
                                )
                                .is_some()
                                {
                                    return EngineAction::None;
                                }
                            }
                        }
                    }
                }

                // Caret Navigation in Edit Controls and Documents (No Shift)
                if key.modifiers.is_empty() || key.modifiers == bit_sr_core::input::KeyModifiers::CONTROL {
                    self.last_selection = None;
                    match key.key {
                        bit_sr_core::input::Key::UpArrow | bit_sr_core::input::Key::DownArrow => {
                            if let Some(ref provider) = self.text_provider {
                                std::thread::sleep(std::time::Duration::from_millis(5));
                                let unit = if key.modifiers == bit_sr_core::input::KeyModifiers::CONTROL {
                                    bit_sr_core::TextUnit::Paragraph
                                } else {
                                    bit_sr_core::TextUnit::Line
                                };
                                let text_opt = provider.get_text_at_caret(unit).or_else(|| {
                                    if unit == bit_sr_core::TextUnit::Paragraph {
                                        provider.get_text_at_caret(bit_sr_core::TextUnit::Line)
                                    } else {
                                        None
                                    }
                                });
                                if let Some(text) = text_opt {
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
                                    std::thread::sleep(std::time::Duration::from_millis(5));
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
                                    std::thread::sleep(std::time::Duration::from_millis(5));
                                    if let Some(text) = provider.get_text_at_caret(bit_sr_core::TextUnit::Character) {
                                        let announcement = if text.is_empty() || text == "\r" || text == "\n" || text == "\r\n" {
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
                        bit_sr_core::input::Key::Home => {
                            if let Some(ref provider) = self.text_provider {
                                std::thread::sleep(std::time::Duration::from_millis(5));
                                if key.modifiers == bit_sr_core::input::KeyModifiers::CONTROL {
                                    // Document start: read line
                                    if let Some(text) = provider.get_text_at_caret(bit_sr_core::TextUnit::Line) {
                                        let announcement = if text.trim().is_empty() {
                                            self.loc.t("format.blank").to_string()
                                        } else {
                                            text.trim_end_matches(&['\r', '\n'][..]).to_string()
                                        };
                                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                                        return EngineAction::Spoke(announcement);
                                    }
                                } else {
                                    // Line start: read character (fallback to line)
                                    let text_opt = provider.get_text_at_caret(bit_sr_core::TextUnit::Character);
                                    if let Some(text) = text_opt {
                                        let announcement = if text.is_empty() || text == "\r" || text == "\n" || text == "\r\n" {
                                            self.loc.t("format.blank").to_string()
                                        } else if text == " " {
                                            self.loc.t("key.space").to_string()
                                        } else {
                                            text
                                        };
                                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                                        return EngineAction::Spoke(announcement);
                                    } else if let Some(line) = provider.get_text_at_caret(bit_sr_core::TextUnit::Line) {
                                        let announcement = if line.trim().is_empty() {
                                            self.loc.t("format.blank").to_string()
                                        } else {
                                            line.trim_end_matches(&['\r', '\n'][..]).to_string()
                                        };
                                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                                        return EngineAction::Spoke(announcement);
                                    }
                                }
                            }
                        }
                        bit_sr_core::input::Key::End => {
                            if let Some(ref provider) = self.text_provider {
                                std::thread::sleep(std::time::Duration::from_millis(5));
                                if key.modifiers == bit_sr_core::input::KeyModifiers::CONTROL {
                                    // Document end: read line
                                    if let Some(text) = provider.get_text_at_caret(bit_sr_core::TextUnit::Line) {
                                        let announcement = if text.trim().is_empty() {
                                            self.loc.t("format.blank").to_string()
                                        } else {
                                            text.trim_end_matches(&['\r', '\n'][..]).to_string()
                                        };
                                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                                        return EngineAction::Spoke(announcement);
                                    }
                                } else {
                                    // Line end: read character (fallback to line)
                                    let text_opt = provider.get_text_at_caret(bit_sr_core::TextUnit::Character);
                                    if let Some(text) = text_opt {
                                        let announcement = if text.is_empty() || text == "\r" || text == "\n" || text == "\r\n" {
                                            self.loc.t("format.blank").to_string()
                                        } else if text == " " {
                                            self.loc.t("key.space").to_string()
                                        } else {
                                            text
                                        };
                                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                                        return EngineAction::Spoke(announcement);
                                    } else if let Some(line) = provider.get_text_at_caret(bit_sr_core::TextUnit::Line) {
                                        let announcement = if line.trim().is_empty() {
                                            self.loc.t("format.blank").to_string()
                                        } else {
                                            line.trim_end_matches(&['\r', '\n'][..]).to_string()
                                        };
                                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                                        return EngineAction::Spoke(announcement);
                                    }
                                }
                            }
                        }
                        bit_sr_core::input::Key::PageUp | bit_sr_core::input::Key::PageDown => {
                            if let Some(ref provider) = self.text_provider {
                                std::thread::sleep(std::time::Duration::from_millis(5));
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
                        bit_sr_core::input::Key::Delete => {
                            if let Some(ref provider) = self.text_provider {
                                std::thread::sleep(std::time::Duration::from_millis(5));
                                let unit = if key.modifiers == bit_sr_core::input::KeyModifiers::CONTROL {
                                    bit_sr_core::TextUnit::Word
                                } else {
                                    bit_sr_core::TextUnit::Character
                                };
                                if let Some(text) = provider.get_text_at_caret(unit) {
                                    if !text.is_empty() {
                                        let announcement = if text == "\r" || text == "\n" || text == "\r\n" {
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
                            let msg = self.loc.t("key.delete");
                            let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                            return EngineAction::Spoke(msg.to_string());
                        }
                        _ => {}
                    }
                }

                // Typing Echo: Printable characters, Space, Enter, Backspace
                if !key.modifiers.intersects(
                    bit_sr_core::input::KeyModifiers::CONTROL
                        | bit_sr_core::input::KeyModifiers::ALT
                        | bit_sr_core::input::KeyModifiers::SUPER
                        | bit_sr_core::input::KeyModifiers::SR,
                ) {
                    match key.key {
                        bit_sr_core::input::Key::Space => {
                            self.last_selection = None;
                            let msg = self.loc.t("key.space");
                            let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                            return EngineAction::Spoke(msg.to_string());
                        }
                        bit_sr_core::input::Key::Enter | bit_sr_core::input::Key::NumpadEnter => {
                            self.last_selection = None;
                            let msg = self.loc.t("key.enter");
                            let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                            return EngineAction::Spoke(msg.to_string());
                        }
                        bit_sr_core::input::Key::Backspace => {
                            self.last_selection = None;
                            let msg = self.loc.t("key.backspace");
                            let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                            return EngineAction::Spoke(msg.to_string());
                        }
                        _ => {
                            let text_to_speak = if let Some(ref text) = key.text {
                                let trimmed = text.trim();
                                if !trimmed.is_empty() {
                                    Some(trimmed.to_string())
                                } else {
                                    None
                                }
                            } else if key.key.is_alphanumeric() {
                                Some(key.key.canonical_name().to_string())
                            } else {
                                None
                            };

                            if let Some(announcement) = text_to_speak {
                                self.last_selection = None;
                                let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                                return EngineAction::Spoke(announcement);
                            }
                        }
                    }
                }

                EngineAction::None
            }

            AccessibilityEvent::Focus(node) => {
                self.last_selection = None;
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

                // Ensure WebController is active if focus enters a web document
                if self.web_controller.is_none() && focused_node.role == bit_sr_core::roles::Role::Document {
                    let buffer = if let Some(ref tp) = self.tree_provider {
                        if let Some(tree) = tp.harvest_tree(14, 1500) {
                            bit_sr_web::Linearizer::compile(&tree, focused_node.id)
                        } else {
                            let mut b = bit_sr_web::VirtualBuffer::new(focused_node.id);
                            b.title = focused_node.name.clone();
                            b
                        }
                    } else {
                        let mut b = bit_sr_web::VirtualBuffer::new(focused_node.id);
                        b.title = focused_node.name.clone();
                        b
                    };
                    self.web_controller = Some(bit_sr_web::WebController::new(buffer));
                }

                // Update WebController automatic Browse/Focus mode state machine
                if let Some(ref mut wc) = self.web_controller {
                    let web_actions = wc.handle_focus_change(focused_node);
                    for action in web_actions {
                        if let bit_sr_web::WebAction::SwitchMode(mode) = action {
                            let mode_msg = match mode {
                                bit_sr_web::NavigationMode::Browse => self.loc.t("web.browse_mode"),
                                bit_sr_web::NavigationMode::Focus => self.loc.t("web.focus_mode"),
                            };
                            let _ = self.speech_hub.speak(mode_msg, SpeechPriority::Now);
                        }
                    }
                }

                if !announcement.trim().is_empty() {
                    let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                    EngineAction::Spoke(announcement)
                } else {
                    EngineAction::None
                }
            }

            AccessibilityEvent::WindowActivated(node) => {
                self.last_selection = None;
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

    /// Detects and announces changes in text selection.
    fn handle_selection_change(&mut self, is_select_all: bool) -> EngineAction {
        if let Some(ref provider) = self.text_provider {
            std::thread::sleep(std::time::Duration::from_millis(25));
            let new_sel = provider.get_selected_text();
            let old_sel = self.last_selection.clone();

            let action = match (&old_sel, &new_sel) {
                (None, Some(new_text)) => {
                    if new_text.is_empty() {
                        EngineAction::None
                    } else {
                        let announcement = if is_select_all && (new_text.len() > 80 || new_text.contains('\n')) {
                            self.loc.t("format.selected_all").to_string()
                        } else {
                            let display = Self::format_selection_content(new_text, &self.loc);
                            self.loc.t_args("format.selected_text", &[("text", &display)])
                        };
                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                        EngineAction::Spoke(announcement)
                    }
                }
                (Some(old_text), Some(new_text)) => {
                    if new_text.is_empty() {
                        let display = Self::format_selection_content(old_text, &self.loc);
                        let announcement = self.loc.t_args("format.unselected_text", &[("text", &display)]);
                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                        EngineAction::Spoke(announcement)
                    } else if old_text == new_text {
                        EngineAction::None
                    } else if new_text.len() > old_text.len() {
                        let added = if new_text.starts_with(old_text) {
                            &new_text[old_text.len()..]
                        } else if new_text.ends_with(old_text) {
                            &new_text[..new_text.len() - old_text.len()]
                        } else {
                            new_text.as_str()
                        };
                        let display = Self::format_selection_content(added, &self.loc);
                        let announcement = self.loc.t_args("format.selected_text", &[("text", &display)]);
                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                        EngineAction::Spoke(announcement)
                    } else {
                        let removed = if old_text.starts_with(new_text) {
                            &old_text[new_text.len()..]
                        } else if old_text.ends_with(new_text) {
                            &old_text[..old_text.len() - new_text.len()]
                        } else {
                            old_text.as_str()
                        };
                        let display = Self::format_selection_content(removed, &self.loc);
                        let announcement = self.loc.t_args("format.unselected_text", &[("text", &display)]);
                        let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                        EngineAction::Spoke(announcement)
                    }
                }
                (Some(old_text), None) => {
                    let display = Self::format_selection_content(old_text, &self.loc);
                    let announcement = self.loc.t_args("format.unselected_text", &[("text", &display)]);
                    let _ = self.speech_hub.speak(&announcement, SpeechPriority::Now);
                    EngineAction::Spoke(announcement)
                }
                (None, None) => EngineAction::None,
            };

            self.last_selection = new_sel;
            return action;
        }
        EngineAction::None
    }

    /// Formats whitespace, newlines, and long strings for selection speech.
    fn format_selection_content(text: &str, loc: &bit_sr_core::LocalizationManager) -> String {
        if text == " " {
            loc.t("key.space").to_string()
        } else if text == "\r" || text == "\n" || text == "\r\n" {
            loc.t("key.enter").to_string()
        } else if text.trim().is_empty() {
            loc.t("format.blank").to_string()
        } else if text.len() > 80 {
            format!("{}...", text[..80].trim_end())
        } else {
            text.to_string()
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

            ScreenReaderCommand::OpenMenu => {
                #[cfg(windows)]
                if let Some(ref tx) = self.event_tx {
                    bit_sr_platform_windows::open_menu_async(tx.clone());
                }
                EngineAction::None
            }

            ScreenReaderCommand::ToggleBrowseMode => {
                if let Some(ref mut wc) = self.web_controller {
                    if wc.buffer.line_count() == 0 {
                        if let Some(ref tp) = self.tree_provider {
                            let root_id = wc.buffer.document_node_id;
                            if let Some(tree) = tp.harvest_tree(14, 1500) {
                                wc.buffer = bit_sr_web::Linearizer::compile(&tree, root_id);
                            }
                        }
                    }
                    let actions = wc.toggle_mode();
                    for action in actions {
                        if let bit_sr_web::WebAction::Speak(ref msg) = action {
                            let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                            return EngineAction::Spoke(msg.clone());
                        }
                    }
                } else {
                    let root_id = self
                        .focus_tracker
                        .current_focus()
                        .map(|n| n.id)
                        .unwrap_or(bit_sr_core::node::NodeId(0));
                    let buffer = if let Some(ref tp) = self.tree_provider {
                        if let Some(tree) = tp.harvest_tree(14, 1500) {
                            bit_sr_web::Linearizer::compile(&tree, root_id)
                        } else {
                            bit_sr_web::VirtualBuffer::new(root_id)
                        }
                    } else {
                        bit_sr_web::VirtualBuffer::new(root_id)
                    };
                    let mut wc = bit_sr_web::WebController::new(buffer);
                    let actions = wc.toggle_mode();
                    self.web_controller = Some(wc);
                    for action in actions {
                        if let bit_sr_web::WebAction::Speak(ref msg) = action {
                            let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                            return EngineAction::Spoke(msg.clone());
                        }
                    }
                    let msg = self.loc.t("web.browse_mode");
                    let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                    return EngineAction::Spoke(msg.to_string());
                }
                EngineAction::None
            }
        }
    }

    /// Runs the reactive event loop until shutdown is signaled or a Quit command is received.
    pub fn run(mut self, event_rx: Receiver<AccessibilityEvent>, shutdown_rx: Receiver<()>) {
        log::info!("Engine coordinator loop active and awaiting events...");
        loop {
            // Process any asynchronous UI events from the Slint GUI
            let ui_events: Vec<bit_sr_ui::UiEvent> = if let Some(ref ui) = self.ui_handle {
                let mut evts = Vec::new();
                while let Some(evt) = ui.try_recv_event() {
                    evts.push(evt);
                }
                evts
            } else {
                Vec::new()
            };

            for evt in ui_events {
                match evt {
                    bit_sr_ui::UiEvent::SettingsSaved(settings) => {
                        let _ = self.speech_hub.set_rate(settings.speech_rate as i32);
                        let _ = self.speech_hub.set_volume(settings.speech_volume as u16);
                    }
                }
            }

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
        current_selection: std::sync::Mutex<Option<String>>,
    }

    impl MockTextProvider {
        fn new(char_text: Option<&str>, word_text: Option<&str>, line_text: Option<&str>) -> Self {
            Self {
                current_character: std::sync::Mutex::new(char_text.map(|s| s.to_string())),
                current_word: std::sync::Mutex::new(word_text.map(|s| s.to_string())),
                current_line: std::sync::Mutex::new(line_text.map(|s| s.to_string())),
                current_selection: std::sync::Mutex::new(None),
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

        fn get_selected_text(&self) -> Option<String> {
            self.current_selection.lock().unwrap().clone()
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

        // Delete with no text provider
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

        // Home key -> Character at line start
        *provider.current_character.lock().unwrap() = Some("a".to_string());
        let key_home = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::Home,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action_home = coordinator.handle_event(AccessibilityEvent::Input(key_home));
        assert_eq!(action_home, EngineAction::Spoke("a".to_string()));

        // Ctrl + Home -> Line at document start
        *provider.current_line.lock().unwrap() = Some("Document First Line\r\n".to_string());
        let key_ctrl_home = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::Home,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::CONTROL,
        );
        let action_ctrl_home = coordinator.handle_event(AccessibilityEvent::Input(key_ctrl_home));
        assert_eq!(action_ctrl_home, EngineAction::Spoke("Document First Line".to_string()));

        // End key -> Character at line end
        *provider.current_character.lock().unwrap() = Some("\n".to_string());
        let key_end = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::End,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action_end = coordinator.handle_event(AccessibilityEvent::Input(key_end));
        assert_eq!(action_end, EngineAction::Spoke("blank".to_string()));

        // Ctrl + End -> Line at document end
        *provider.current_line.lock().unwrap() = Some("Document Last Line".to_string());
        let key_ctrl_end = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::End,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::CONTROL,
        );
        let action_ctrl_end = coordinator.handle_event(AccessibilityEvent::Input(key_ctrl_end));
        assert_eq!(action_ctrl_end, EngineAction::Spoke("Document Last Line".to_string()));

        // Delete key with provider -> Character at caret
        *provider.current_character.lock().unwrap() = Some("x".to_string());
        let key_del = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::Delete,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action_del = coordinator.handle_event(AccessibilityEvent::Input(key_del));
        assert_eq!(action_del, EngineAction::Spoke("x".to_string()));

        assert_eq!(
            history.get_spoken_history(),
            vec![
                "hello world",
                "h",
                "space",
                "blank",
                "hello",
                "a",
                "Document First Line",
                "blank",
                "Document Last Line",
                "x",
            ]
        );
    }

    #[test]
    fn test_coordinator_selection_changes() {
        let mut hub = SpeechHub::new();
        let mock = MockSynthesizer::new();
        let history = mock.clone();
        hub.register_driver(Box::new(mock));

        let mut coordinator = EngineCoordinator::new(hub);
        let provider = std::sync::Arc::new(MockTextProvider::new(None, None, None));
        coordinator.set_text_provider(provider.clone());

        // Shift + RightArrow: Select first character "H"
        *provider.current_selection.lock().unwrap() = Some("H".to_string());
        let key_shift_right = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::RightArrow,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::SHIFT,
        );
        let action_sel1 = coordinator.handle_event(AccessibilityEvent::Input(key_shift_right.clone()));
        assert_eq!(action_sel1, EngineAction::Spoke("selected H".to_string()));

        // Shift + RightArrow: Expand selection to "He" -> Announces added character "e"
        *provider.current_selection.lock().unwrap() = Some("He".to_string());
        let action_sel2 = coordinator.handle_event(AccessibilityEvent::Input(key_shift_right));
        assert_eq!(action_sel2, EngineAction::Spoke("selected e".to_string()));

        // Shift + LeftArrow: Shrink selection back to "H" -> Announces unselected "e"
        *provider.current_selection.lock().unwrap() = Some("H".to_string());
        let key_shift_left = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::LeftArrow,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::SHIFT,
        );
        let action_unsel1 = coordinator.handle_event(AccessibilityEvent::Input(key_shift_left.clone()));
        assert_eq!(action_unsel1, EngineAction::Spoke("unselected e".to_string()));

        // Shift + LeftArrow: Selection completely cleared -> Announces unselected "H"
        *provider.current_selection.lock().unwrap() = None;
        let action_unsel2 = coordinator.handle_event(AccessibilityEvent::Input(key_shift_left));
        assert_eq!(action_unsel2, EngineAction::Spoke("unselected H".to_string()));

        // Ctrl + A: Select All (multiline text)
        *provider.current_selection.lock().unwrap() = Some("Line 1\r\nLine 2\r\nLine 3\r\n".to_string());
        let key_ctrl_a = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::A,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::CONTROL,
        );
        let action_ctrl_a = coordinator.handle_event(AccessibilityEvent::Input(key_ctrl_a));
        assert_eq!(action_ctrl_a, EngineAction::Spoke("selected all".to_string()));

        assert_eq!(
            history.get_spoken_history(),
            vec!["selected H", "selected e", "unselected e", "unselected H", "selected all"]
        );
    }

    #[test]
    fn test_coordinator_menu_action() {
        let mut hub = SpeechHub::new();
        let mock = MockSynthesizer::new();
        let history = mock.clone();
        hub.register_driver(Box::new(mock));

        let mut coordinator = EngineCoordinator::new(hub);

        // Mute via menu action
        let action = coordinator.handle_event(AccessibilityEvent::MenuAction(bit_sr_core::menu::MenuAction::SetSpeechModeMute));
        assert_eq!(action, EngineAction::Spoke("Speech muted".to_string()));
        assert_eq!(coordinator.command_dispatcher.speech_mode, SpeechMode::Mute);

        // Talk via menu action
        let action2 = coordinator.handle_event(AccessibilityEvent::MenuAction(bit_sr_core::menu::MenuAction::SetSpeechModeTalk));
        assert_eq!(action2, EngineAction::Spoke("Speech on".to_string()));
        assert_eq!(coordinator.command_dispatcher.speech_mode, SpeechMode::Talk);

        // Quit via menu action
        let action3 = coordinator.handle_event(AccessibilityEvent::MenuAction(bit_sr_core::menu::MenuAction::Quit));
        assert_eq!(action3, EngineAction::Quit);

        assert_eq!(history.get_spoken_history(), vec!["Speech muted", "Speech on", "Exiting bit_sr"]);
    }

    #[test]
    fn test_coordinator_web_virtual_buffer_navigation() {
        let mut hub = SpeechHub::new();
        let mock = MockSynthesizer::new();
        let history = mock.clone();
        hub.register_driver(Box::new(mock));

        let mut coordinator = EngineCoordinator::new(hub);

        // Linearized web buffer
        let lines = vec![
            bit_sr_web::BufferLine::new(
                0,
                vec![bit_sr_web::TextRun::new_heading(bit_sr_core::node::NodeId(1), "Welcome to the Web", 1)],
            ),
            bit_sr_web::BufferLine::new(
                1,
                vec![bit_sr_web::TextRun::new_text(bit_sr_core::node::NodeId(2), "Article content paragraph.")],
            ),
            bit_sr_web::BufferLine::new(
                2,
                vec![bit_sr_web::TextRun::new_button(bit_sr_core::node::NodeId(3), "Submit Form")],
            ),
        ];
        let buffer = bit_sr_web::VirtualBuffer::with_lines(bit_sr_core::node::NodeId(100), lines);
        coordinator.set_web_buffer(buffer);

        // Down Arrow: reads next line in virtual buffer
        let key_down = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::DownArrow,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action1 = coordinator.handle_event(AccessibilityEvent::Input(key_down));
        assert_eq!(action1, EngineAction::Spoke("Article content paragraph.".to_string()));

        // Single-letter quick nav: B jumps to button
        let key_b = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::B,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action2 = coordinator.handle_event(AccessibilityEvent::Input(key_b));
        assert_eq!(action2, EngineAction::Spoke("Submit Form, button".to_string()));

        // Toggle Browse Mode command (SR + Space) -> switches to Focus Mode
        let action_toggle = coordinator.toggle_browse_mode();
        assert_eq!(action_toggle, EngineAction::Spoke("Focus mode".to_string()));

        // Escape key in Focus Mode -> switches back to Browse Mode
        let key_esc = bit_sr_core::input::KeyEvent::new(
            bit_sr_core::input::Key::Escape,
            bit_sr_core::input::KeyAction::Down,
            bit_sr_core::input::KeyModifiers::empty(),
        );
        let action_esc = coordinator.handle_event(AccessibilityEvent::Input(key_esc));
        assert_eq!(action_esc, EngineAction::Spoke("Browse mode".to_string()));

        assert_eq!(
            history.get_spoken_history(),
            vec![
                "Article content paragraph.",
                "Submit Form, button",
                "Focus mode",
                "Browse mode"
            ]
        );
    }
}


