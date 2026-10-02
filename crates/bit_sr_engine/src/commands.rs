//! Dynamic Screen Reader Shortcut Registry and Gesture Mapping System.
//! Inspired by NVDA's inputCore gesture maps with runtime rebinding,
//! input help mode, and zero-allocation key matching.

use bit_sr_core::input::{GestureParseError, InputGesture, Key, KeyAction, KeyEvent, KeyModifiers};
use std::collections::HashMap;

/// Speech output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpeechMode {
    #[default]
    Talk,
    Mute,
}

/// Commands triggered by screen reader shortcut key combinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScreenReaderCommand {
    AnnounceTitle,
    RepeatFocus,
    ToggleSpeechMode,
    VolumeUp,
    VolumeDown,
    RateFaster,
    RateSlower,
    ToggleInputHelp,
    OpenMenu,
    ToggleBrowseMode,
    Quit,

    // Review Cursor Commands (Desktop Numpad & Laptop)
    ReviewPreviousLine,
    ReviewCurrentLine,
    ReviewNextLine,
    ReviewPreviousWord,
    ReviewCurrentWord,
    ReviewNextWord,
    ReviewPreviousCharacter,
    ReviewCurrentCharacter,
    ReviewNextCharacter,
    ReviewTop,
    ReviewBottom,
    ReviewStartOfLine,
    ReviewEndOfLine,
}

impl ScreenReaderCommand {
    /// String identifier for configuration files and plugins (e.g. "core.announce_title").
    pub fn id(&self) -> &'static str {
        match self {
            Self::AnnounceTitle => "core.announce_title",
            Self::RepeatFocus => "core.repeat_focus",
            Self::ToggleSpeechMode => "speech.toggle_mode",
            Self::VolumeUp => "speech.volume_up",
            Self::VolumeDown => "speech.volume_down",
            Self::RateFaster => "speech.rate_faster",
            Self::RateSlower => "speech.rate_slower",
            Self::ToggleInputHelp => "help.toggle_input_help",
            Self::OpenMenu => "core.open_menu",
            Self::ToggleBrowseMode => "web.toggle_browse_mode",
            Self::Quit => "app.quit",
            Self::ReviewPreviousLine => "review.prev_line",
            Self::ReviewCurrentLine => "review.curr_line",
            Self::ReviewNextLine => "review.next_line",
            Self::ReviewPreviousWord => "review.prev_word",
            Self::ReviewCurrentWord => "review.curr_word",
            Self::ReviewNextWord => "review.next_word",
            Self::ReviewPreviousCharacter => "review.prev_char",
            Self::ReviewCurrentCharacter => "review.curr_char",
            Self::ReviewNextCharacter => "review.next_char",
            Self::ReviewTop => "review.top",
            Self::ReviewBottom => "review.bottom",
            Self::ReviewStartOfLine => "review.start_line",
            Self::ReviewEndOfLine => "review.end_line",
        }
    }

    /// Functional category (e.g. "System", "Speech", "Help", "Web", "Review").
    pub fn category(&self) -> &'static str {
        match self {
            Self::AnnounceTitle | Self::RepeatFocus | Self::OpenMenu | Self::Quit => "System",
            Self::ToggleSpeechMode
            | Self::VolumeUp
            | Self::VolumeDown
            | Self::RateFaster
            | Self::RateSlower => "Speech",
            Self::ToggleInputHelp => "Help",
            Self::ToggleBrowseMode => "Web",
            Self::ReviewPreviousLine
            | Self::ReviewCurrentLine
            | Self::ReviewNextLine
            | Self::ReviewPreviousWord
            | Self::ReviewCurrentWord
            | Self::ReviewNextWord
            | Self::ReviewPreviousCharacter
            | Self::ReviewCurrentCharacter
            | Self::ReviewNextCharacter
            | Self::ReviewTop
            | Self::ReviewBottom
            | Self::ReviewStartOfLine
            | Self::ReviewEndOfLine => "Review",
        }
    }

    /// Translation key for the command in i18n catalogs.
    pub fn i18n_key(&self) -> &'static str {
        match self {
            Self::AnnounceTitle => "cmd.announce_title",
            Self::RepeatFocus => "cmd.repeat_focus",
            Self::ToggleSpeechMode => "cmd.toggle_speech_mode",
            Self::VolumeUp => "cmd.volume_up",
            Self::VolumeDown => "cmd.volume_down",
            Self::RateFaster => "cmd.rate_faster",
            Self::RateSlower => "cmd.rate_slower",
            Self::ToggleInputHelp => "cmd.toggle_input_help",
            Self::OpenMenu => "cmd.open_menu",
            Self::ToggleBrowseMode => "cmd.toggle_browse_mode",
            Self::Quit => "cmd.quit",
            Self::ReviewPreviousLine => "cmd.review_prev_line",
            Self::ReviewCurrentLine => "cmd.review_curr_line",
            Self::ReviewNextLine => "cmd.review_next_line",
            Self::ReviewPreviousWord => "cmd.review_prev_word",
            Self::ReviewCurrentWord => "cmd.review_curr_word",
            Self::ReviewNextWord => "cmd.review_next_word",
            Self::ReviewPreviousCharacter => "cmd.review_prev_char",
            Self::ReviewCurrentCharacter => "cmd.review_curr_char",
            Self::ReviewNextCharacter => "cmd.review_next_char",
            Self::ReviewTop => "cmd.review_top",
            Self::ReviewBottom => "cmd.review_bottom",
            Self::ReviewStartOfLine => "cmd.review_start_line",
            Self::ReviewEndOfLine => "cmd.review_end_line",
        }
    }

    /// Localized human-friendly display name for UI and Input Help mode.
    pub fn display_name_localized(&self, loc: &bit_sr_core::LocalizationManager) -> &'static str {
        loc.t(self.i18n_key())
    }

    /// Human-friendly display name for UI and Input Help mode.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AnnounceTitle => "Announce Window Title",
            Self::RepeatFocus => "Repeat Focused Element",
            Self::ToggleSpeechMode => "Toggle Speech Mode (Talk / Mute)",
            Self::VolumeUp => "Increase Volume",
            Self::VolumeDown => "Decrease Volume",
            Self::RateFaster => "Increase Speech Rate",
            Self::RateSlower => "Decrease Speech Rate",
            Self::ToggleInputHelp => "Toggle Input Help Mode",
            Self::OpenMenu => "Open bit_sr Menu",
            Self::ToggleBrowseMode => "Toggle Browse / Focus Mode",
            Self::Quit => "Exit bit_sr",
            Self::ReviewPreviousLine => "Review Previous Line",
            Self::ReviewCurrentLine => "Review Current Line",
            Self::ReviewNextLine => "Review Next Line",
            Self::ReviewPreviousWord => "Review Previous Word",
            Self::ReviewCurrentWord => "Review Current Word",
            Self::ReviewNextWord => "Review Next Word",
            Self::ReviewPreviousCharacter => "Review Previous Character",
            Self::ReviewCurrentCharacter => "Review Current Character",
            Self::ReviewNextCharacter => "Review Next Character",
            Self::ReviewTop => "Review Top Line",
            Self::ReviewBottom => "Review Bottom Line",
            Self::ReviewStartOfLine => "Review Start of Line",
            Self::ReviewEndOfLine => "Review End of Line",
        }
    }

    /// Parses command from a string ID.
    pub fn from_id(id: &str) -> Option<Self> {
        match id.trim().to_lowercase().as_str() {
            "core.announce_title" | "announcetitle" | "title" => Some(Self::AnnounceTitle),
            "core.repeat_focus" | "repeatfocus" | "focus" => Some(Self::RepeatFocus),
            "speech.toggle_mode" | "togglespeechmode" | "speechmode" => {
                Some(Self::ToggleSpeechMode)
            }
            "speech.volume_up" | "volumeup" => Some(Self::VolumeUp),
            "speech.volume_down" | "volumedown" => Some(Self::VolumeDown),
            "speech.rate_faster" | "ratefaster" | "rateup" => Some(Self::RateFaster),
            "speech.rate_slower" | "rateslower" | "ratedown" => Some(Self::RateSlower),
            "help.toggle_input_help" | "toggleinputhelp" | "inputhelp" => {
                Some(Self::ToggleInputHelp)
            }
            "core.open_menu" | "openmenu" | "menu" => Some(Self::OpenMenu),
            "web.toggle_browse_mode" | "togglebrowsemode" | "browsemode" => {
                Some(Self::ToggleBrowseMode)
            }
            "app.quit" | "quit" | "exit" => Some(Self::Quit),
            "review.prev_line" | "review_prev_line" => Some(Self::ReviewPreviousLine),
            "review.curr_line" | "review_curr_line" => Some(Self::ReviewCurrentLine),
            "review.next_line" | "review_next_line" => Some(Self::ReviewNextLine),
            "review.prev_word" | "review_prev_word" => Some(Self::ReviewPreviousWord),
            "review.curr_word" | "review_curr_word" => Some(Self::ReviewCurrentWord),
            "review.next_word" | "review_next_word" => Some(Self::ReviewNextWord),
            "review.prev_char" | "review_prev_char" => Some(Self::ReviewPreviousCharacter),
            "review.curr_char" | "review_curr_char" => Some(Self::ReviewCurrentCharacter),
            "review.next_char" | "review_next_char" => Some(Self::ReviewNextCharacter),
            "review.top" | "review_top" => Some(Self::ReviewTop),
            "review.bottom" | "review_bottom" => Some(Self::ReviewBottom),
            "review.start_line" | "review_start_line" => Some(Self::ReviewStartOfLine),
            "review.end_line" | "review_end_line" => Some(Self::ReviewEndOfLine),
            _ => None,
        }
    }
}

/// Dynamic Gesture Map maintaining key-to-command bindings.
#[derive(Debug, Clone)]
pub struct GestureMap {
    bindings: HashMap<InputGesture, ScreenReaderCommand>,
}

impl Default for GestureMap {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl GestureMap {
    /// Creates a GestureMap pre-populated with standard default bindings.
    pub fn with_defaults() -> Self {
        let mut map = Self {
            bindings: HashMap::new(),
        };
        map.register_defaults();
        map
    }

    /// Populates built-in default bindings.
    pub fn register_defaults(&mut self) {
        // SR + T -> Announce Title
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::T),
            ScreenReaderCommand::AnnounceTitle,
        );

        // SR + Tab -> Repeat Focus
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::Tab),
            ScreenReaderCommand::RepeatFocus,
        );

        // SR + S -> Toggle Speech Mode
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::S),
            ScreenReaderCommand::ToggleSpeechMode,
        );

        // Ctrl + SR + Up / Down -> Volume Up / Down
        self.bind(
            InputGesture::new(KeyModifiers::CONTROL | KeyModifiers::SR, Key::UpArrow),
            ScreenReaderCommand::VolumeUp,
        );
        self.bind(
            InputGesture::new(KeyModifiers::CONTROL | KeyModifiers::SR, Key::DownArrow),
            ScreenReaderCommand::VolumeDown,
        );

        // Ctrl + SR + Right / Left -> Rate Faster / Slower
        self.bind(
            InputGesture::new(KeyModifiers::CONTROL | KeyModifiers::SR, Key::RightArrow),
            ScreenReaderCommand::RateFaster,
        );
        self.bind(
            InputGesture::new(KeyModifiers::CONTROL | KeyModifiers::SR, Key::LeftArrow),
            ScreenReaderCommand::RateSlower,
        );

        // SR + 1 (Number row & Numpad 1) -> Toggle Input Help Mode
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::Num1),
            ScreenReaderCommand::ToggleInputHelp,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::Numpad1),
            ScreenReaderCommand::ToggleInputHelp,
        );

        // SR + M -> Open Menu
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::M),
            ScreenReaderCommand::OpenMenu,
        );

        // SR + Space -> Toggle Browse / Focus Mode
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::Space),
            ScreenReaderCommand::ToggleBrowseMode,
        );

        // Ctrl + Alt + Q and SR + Q -> Quit
        self.bind(
            InputGesture::new(KeyModifiers::CONTROL | KeyModifiers::ALT, Key::Q),
            ScreenReaderCommand::Quit,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::Q),
            ScreenReaderCommand::Quit,
        );

        // ====================================================================
        // Text Review Cursor Commands (Desktop Numpad Layout - NumLock OFF)
        // ====================================================================
        self.bind(
            InputGesture::new(KeyModifiers::empty(), Key::Numpad7),
            ScreenReaderCommand::ReviewPreviousLine,
        );
        self.bind(
            InputGesture::new(KeyModifiers::empty(), Key::Numpad8),
            ScreenReaderCommand::ReviewCurrentLine,
        );
        self.bind(
            InputGesture::new(KeyModifiers::empty(), Key::Numpad9),
            ScreenReaderCommand::ReviewNextLine,
        );
        self.bind(
            InputGesture::new(KeyModifiers::empty(), Key::Numpad4),
            ScreenReaderCommand::ReviewPreviousWord,
        );
        self.bind(
            InputGesture::new(KeyModifiers::empty(), Key::Numpad5),
            ScreenReaderCommand::ReviewCurrentWord,
        );
        self.bind(
            InputGesture::new(KeyModifiers::empty(), Key::Numpad6),
            ScreenReaderCommand::ReviewNextWord,
        );
        self.bind(
            InputGesture::new(KeyModifiers::empty(), Key::Numpad1),
            ScreenReaderCommand::ReviewPreviousCharacter,
        );
        self.bind(
            InputGesture::new(KeyModifiers::empty(), Key::Numpad2),
            ScreenReaderCommand::ReviewCurrentCharacter,
        );
        self.bind(
            InputGesture::new(KeyModifiers::empty(), Key::Numpad3),
            ScreenReaderCommand::ReviewNextCharacter,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SHIFT, Key::Numpad7),
            ScreenReaderCommand::ReviewTop,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SHIFT, Key::Numpad9),
            ScreenReaderCommand::ReviewBottom,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SHIFT, Key::Numpad1),
            ScreenReaderCommand::ReviewStartOfLine,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SHIFT, Key::Numpad3),
            ScreenReaderCommand::ReviewEndOfLine,
        );

        // ====================================================================
        // Text Review Cursor Commands (Laptop Layout - SR modifier)
        // ====================================================================
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::UpArrow),
            ScreenReaderCommand::ReviewPreviousLine,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR | KeyModifiers::SHIFT, Key::Period),
            ScreenReaderCommand::ReviewCurrentLine,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::DownArrow),
            ScreenReaderCommand::ReviewNextLine,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR | KeyModifiers::CONTROL, Key::LeftArrow),
            ScreenReaderCommand::ReviewPreviousWord,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR | KeyModifiers::CONTROL, Key::Period),
            ScreenReaderCommand::ReviewCurrentWord,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR | KeyModifiers::CONTROL, Key::RightArrow),
            ScreenReaderCommand::ReviewNextWord,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::LeftArrow),
            ScreenReaderCommand::ReviewPreviousCharacter,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::Period),
            ScreenReaderCommand::ReviewCurrentCharacter,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::RightArrow),
            ScreenReaderCommand::ReviewNextCharacter,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR | KeyModifiers::CONTROL, Key::Home),
            ScreenReaderCommand::ReviewTop,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR | KeyModifiers::CONTROL, Key::End),
            ScreenReaderCommand::ReviewBottom,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::Home),
            ScreenReaderCommand::ReviewStartOfLine,
        );
        self.bind(
            InputGesture::new(KeyModifiers::SR, Key::End),
            ScreenReaderCommand::ReviewEndOfLine,
        );
    }

    /// Binds an InputGesture to a ScreenReaderCommand.
    pub fn bind(&mut self, gesture: InputGesture, cmd: ScreenReaderCommand) {
        self.bindings.insert(gesture, cmd);
    }

    /// Binds a gesture by string representation (e.g. "sr+w" -> AnnounceTitle).
    pub fn bind_str(
        &mut self,
        gesture_str: &str,
        cmd: ScreenReaderCommand,
    ) -> Result<(), GestureParseError> {
        let gesture = InputGesture::parse(gesture_str)?;
        self.bind(gesture, cmd);
        Ok(())
    }

    /// Unbinds a gesture.
    pub fn unbind(&mut self, gesture: &InputGesture) -> Option<ScreenReaderCommand> {
        self.bindings.remove(gesture)
    }

    /// Unbinds a gesture by string representation.
    pub fn unbind_str(
        &mut self,
        gesture_str: &str,
    ) -> Result<Option<ScreenReaderCommand>, GestureParseError> {
        let gesture = InputGesture::parse(gesture_str)?;
        Ok(self.unbind(&gesture))
    }

    /// Looks up the command bound to a gesture.
    pub fn lookup(&self, gesture: &InputGesture) -> Option<ScreenReaderCommand> {
        self.bindings.get(gesture).copied()
    }

    /// Finds all gestures bound to a given command.
    pub fn get_gestures_for_command(&self, cmd: ScreenReaderCommand) -> Vec<InputGesture> {
        self.bindings
            .iter()
            .filter(|&(_, &c)| c == cmd)
            .map(|(&g, _)| g)
            .collect()
    }
}

pub struct CommandDispatcher {
    pub speech_mode: SpeechMode,
    pub input_help_active: bool,
    pub gesture_map: GestureMap,
}

impl CommandDispatcher {
    pub fn new() -> Self {
        Self {
            speech_mode: SpeechMode::Talk,
            input_help_active: false,
            gesture_map: GestureMap::with_defaults(),
        }
    }

    /// Evaluates a key event using the dynamic GestureMap.
    pub fn process_key(&mut self, key: &KeyEvent) -> Option<ScreenReaderCommand> {
        if key.action != KeyAction::Down {
            return None;
        }

        // Fallback key mapping if key was constructed with Key::Unknown
        let effective_key = if key.key == Key::Unknown {
            match key.vk_code {
                0x54 => Key::T,
                0x51 => Key::Q,
                0x53 => Key::S,
                0x4D => Key::M,
                0x09 => Key::Tab,
                0x26 => Key::UpArrow,
                0x28 => Key::DownArrow,
                0x27 => Key::RightArrow,
                0x25 => Key::LeftArrow,
                0x31 => Key::Num1,
                _ => key.key,
            }
        } else {
            key.key
        };

        let gesture = InputGesture::new(key.modifiers, effective_key);

        if let Some(cmd) = self.gesture_map.lookup(&gesture) {
            match cmd {
                ScreenReaderCommand::ToggleSpeechMode => {
                    self.speech_mode = match self.speech_mode {
                        SpeechMode::Talk => SpeechMode::Mute,
                        SpeechMode::Mute => SpeechMode::Talk,
                    };
                }
                ScreenReaderCommand::ToggleInputHelp => {
                    self.input_help_active = !self.input_help_active;
                }
                _ => {}
            }
            Some(cmd)
        } else {
            None
        }
    }
}

impl Default for CommandDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sr_shortcuts() {
        let mut dispatcher = CommandDispatcher::new();

        // Insert + T -> AnnounceTitle
        let key_t = KeyEvent {
            key: Key::T,
            vk_code: 0x54,
            scan_code: 0,
            is_extended: false,
            is_injected: false,
            action: KeyAction::Down,
            modifiers: KeyModifiers::INSERT,
            text: None,
        };
        assert_eq!(
            dispatcher.process_key(&key_t),
            Some(ScreenReaderCommand::AnnounceTitle)
        );

        // Ctrl + Alt + Q -> Quit
        let key_quit = KeyEvent {
            key: Key::Q,
            vk_code: 0x51,
            scan_code: 0,
            is_extended: false,
            is_injected: false,
            action: KeyAction::Down,
            modifiers: KeyModifiers::CONTROL | KeyModifiers::ALT,
            text: None,
        };
        assert_eq!(
            dispatcher.process_key(&key_quit),
            Some(ScreenReaderCommand::Quit)
        );

        // CapsLock + S -> Toggle Speech Mode
        let key_s = KeyEvent {
            key: Key::S,
            vk_code: 0x53,
            scan_code: 0,
            is_extended: false,
            is_injected: false,
            action: KeyAction::Down,
            modifiers: KeyModifiers::SR,
            text: None,
        };
        assert_eq!(
            dispatcher.process_key(&key_s),
            Some(ScreenReaderCommand::ToggleSpeechMode)
        );
        assert_eq!(dispatcher.speech_mode, SpeechMode::Mute);
    }

    #[test]
    fn test_dynamic_gesture_rebinding() {
        let mut dispatcher = CommandDispatcher::new();

        // Rebind "sr+w" to AnnounceTitle
        dispatcher
            .gesture_map
            .bind_str("sr+w", ScreenReaderCommand::AnnounceTitle)
            .unwrap();

        let key_w = KeyEvent::new(Key::W, KeyAction::Down, KeyModifiers::SR);
        assert_eq!(
            dispatcher.process_key(&key_w),
            Some(ScreenReaderCommand::AnnounceTitle)
        );

        // Unbind "sr+t"
        dispatcher.gesture_map.unbind_str("sr+t").unwrap();
        let key_t = KeyEvent::new(Key::T, KeyAction::Down, KeyModifiers::SR);
        assert_eq!(dispatcher.process_key(&key_t), None);
    }

    #[test]
    fn test_input_help_toggle() {
        let mut dispatcher = CommandDispatcher::new();
        assert!(!dispatcher.input_help_active);

        let key_sr_1 = KeyEvent::new(Key::Num1, KeyAction::Down, KeyModifiers::SR);
        let cmd = dispatcher.process_key(&key_sr_1);
        assert_eq!(cmd, Some(ScreenReaderCommand::ToggleInputHelp));
        assert!(dispatcher.input_help_active);

        // Press again to toggle off
        let cmd2 = dispatcher.process_key(&key_sr_1);
        assert_eq!(cmd2, Some(ScreenReaderCommand::ToggleInputHelp));
        assert!(!dispatcher.input_help_active);
    }

    #[test]
    fn test_open_menu_shortcut() {
        let mut dispatcher = CommandDispatcher::new();
        let key_sr_m = KeyEvent::new(Key::M, KeyAction::Down, KeyModifiers::SR);
        let cmd = dispatcher.process_key(&key_sr_m);
        assert_eq!(cmd, Some(ScreenReaderCommand::OpenMenu));
    }
}
