//! Screen Reader Commands and Shortcut Interception.

use bit_sr_core::input::{KeyAction, KeyEvent, KeyModifiers};

/// Speech output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpeechMode {
    #[default]
    Talk,
    Mute,
}

/// Commands triggered by screen reader shortcut key combinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenReaderCommand {
    AnnounceTitle,
    RepeatFocus,
    ToggleSpeechMode,
    VolumeUp,
    VolumeDown,
    RateFaster,
    RateSlower,
    Quit,
}

pub struct CommandDispatcher {
    pub speech_mode: SpeechMode,
}

impl CommandDispatcher {
    pub fn new() -> Self {
        Self {
            speech_mode: SpeechMode::Talk,
        }
    }

    /// Evaluates a key event and checks if it matches a screen reader shortcut command.
    pub fn process_key(&mut self, key: &KeyEvent) -> Option<ScreenReaderCommand> {
        if key.action != KeyAction::Down {
            return None;
        }

        let is_sr_modifier = key.modifiers.contains(KeyModifiers::INSERT)
            || key.modifiers.contains(KeyModifiers::CAPSLOCK);

        let is_ctrl_alt = key.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::ALT);

        // Emergency / Standard Quit: Ctrl + Alt + Q or Insert/CapsLock + Q
        if (is_ctrl_alt || is_sr_modifier) && key.vk_code == 0x51 {
            return Some(ScreenReaderCommand::Quit);
        }

        if !is_sr_modifier {
            return None;
        }

        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.vk_code {
            // T: Speak Window Title
            0x54 => Some(ScreenReaderCommand::AnnounceTitle),

            // Tab: Repeat Focus
            0x09 => Some(ScreenReaderCommand::RepeatFocus),

            // S: Toggle Speech Mode (Talk / Mute)
            0x53 => {
                self.speech_mode = match self.speech_mode {
                    SpeechMode::Talk => SpeechMode::Mute,
                    SpeechMode::Mute => SpeechMode::Talk,
                };
                Some(ScreenReaderCommand::ToggleSpeechMode)
            }

            // Arrow Keys with Ctrl: Volume & Rate
            0x26 if has_ctrl => Some(ScreenReaderCommand::VolumeUp),
            0x28 if has_ctrl => Some(ScreenReaderCommand::VolumeDown),
            0x27 if has_ctrl => Some(ScreenReaderCommand::RateFaster),
            0x25 if has_ctrl => Some(ScreenReaderCommand::RateSlower),

            _ => None,
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
            vk_code: 0x54,
            scan_code: 0,
            is_extended: false,
            is_injected: false,
            action: KeyAction::Down,
            modifiers: KeyModifiers::INSERT,
            text: None,
        };
        assert_eq!(dispatcher.process_key(&key_t), Some(ScreenReaderCommand::AnnounceTitle));

        // Ctrl + Alt + Q -> Quit
        let key_quit = KeyEvent {
            vk_code: 0x51,
            scan_code: 0,
            is_extended: false,
            is_injected: false,
            action: KeyAction::Down,
            modifiers: KeyModifiers::CONTROL | KeyModifiers::ALT,
            text: None,
        };
        assert_eq!(dispatcher.process_key(&key_quit), Some(ScreenReaderCommand::Quit));

        // Insert + S -> Toggle Speech Mode
        let key_s = KeyEvent {
            vk_code: 0x53,
            scan_code: 0,
            is_extended: false,
            is_injected: false,
            action: KeyAction::Down,
            modifiers: KeyModifiers::INSERT,
            text: None,
        };
        assert_eq!(dispatcher.process_key(&key_s), Some(ScreenReaderCommand::ToggleSpeechMode));
        assert_eq!(dispatcher.speech_mode, SpeechMode::Mute);
    }
}
