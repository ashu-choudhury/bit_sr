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
}

impl EngineCoordinator {
    pub fn new(speech_hub: SpeechHub) -> Self {
        Self {
            speech_hub,
            focus_tracker: FocusTracker::new(),
            command_dispatcher: CommandDispatcher::new(),
            formatter_context: FormatterContext::default(),
        }
    }

    /// Handles a single incoming event from the platform.
    pub fn handle_event(&mut self, event: AccessibilityEvent) -> EngineAction {
        match event {
            AccessibilityEvent::SpeechInterrupt => {
                let _ = self.speech_hub.interrupt();
                EngineAction::Interrupted
            }

            AccessibilityEvent::Input(key) => {
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
                        announcement.push_str(&format!("{}, window, ", title));
                    }
                }

                let node_text = SpeechFormatter::format_focus(focused_node, &mut self.formatter_context);
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
                    let announcement = format!("{}, window", t);
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
                        if let Some(state_str) = SpeechFormatter::format_state_change(state, is_set) {
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
                let _ = self.speech_hub.speak("Exiting bit_sr", SpeechPriority::Now);
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
                let msg = format!("{}, window", title);
                let _ = self.speech_hub.speak(&msg, SpeechPriority::Now);
                EngineAction::Spoke(msg)
            }

            ScreenReaderCommand::RepeatFocus => {
                if let Some(focused) = self.focus_tracker.current_focus() {
                    let text = SpeechFormatter::format_focus(focused, &mut self.formatter_context);
                    let _ = self.speech_hub.speak(&text, SpeechPriority::Now);
                    EngineAction::Spoke(text)
                } else {
                    let msg = "No element focused";
                    let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                    EngineAction::Spoke(msg.to_string())
                }
            }

            ScreenReaderCommand::ToggleSpeechMode => {
                let msg = match self.command_dispatcher.speech_mode {
                    SpeechMode::Talk => "Speech on",
                    SpeechMode::Mute => "Speech muted",
                };
                let _ = self.speech_hub.speak(msg, SpeechPriority::Now);
                EngineAction::Spoke(msg.to_string())
            }

            ScreenReaderCommand::VolumeUp => {
                let vol = (self.speech_hub.get_volume() + 10).min(100);
                let _ = self.speech_hub.set_volume(vol);
                let msg = format!("Volume {}", vol);
                let _ = self.speech_hub.speak(&msg, SpeechPriority::Now);
                EngineAction::Spoke(msg)
            }

            ScreenReaderCommand::VolumeDown => {
                let vol = self.speech_hub.get_volume().saturating_sub(10);
                let _ = self.speech_hub.set_volume(vol);
                let msg = format!("Volume {}", vol);
                let _ = self.speech_hub.speak(&msg, SpeechPriority::Now);
                EngineAction::Spoke(msg)
            }

            ScreenReaderCommand::RateFaster => {
                let rate = (self.speech_hub.get_rate() + 1).min(10);
                let _ = self.speech_hub.set_rate(rate);
                let msg = format!("Rate {}", rate);
                let _ = self.speech_hub.speak(&msg, SpeechPriority::Now);
                EngineAction::Spoke(msg)
            }

            ScreenReaderCommand::RateSlower => {
                let rate = (self.speech_hub.get_rate() - 1).max(-10);
                let _ = self.speech_hub.set_rate(rate);
                let msg = format!("Rate {}", rate);
                let _ = self.speech_hub.speak(&msg, SpeechPriority::Now);
                EngineAction::Spoke(msg)
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
}
