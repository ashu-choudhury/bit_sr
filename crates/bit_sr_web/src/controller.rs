//! Web Controller: Orchestrates VirtualBuffer cursor traversal,
//! automatic Browse/Focus mode state machine, and quick navigation.

use crate::buffer::{NavigationMode, VirtualBuffer};
use crate::quick_nav::{QuickNav, QuickNavKey};
use bit_sr_core::actions::AccessibleAction;
use bit_sr_core::input::{Key, KeyEvent, KeyModifiers};
use bit_sr_core::node::{AccessibleNode, NodeId};
use bit_sr_core::roles::Role;

/// Action emitted by the web controller for the engine coordinator to execute.
#[derive(Debug, Clone, PartialEq)]
pub enum WebAction {
    /// No action required.
    None,
    /// Speak a notification or element description.
    Speak(String),
    /// Play a mode-switch earcon sound.
    PlaySound(&'static str),
    /// Explicit mode switch event.
    SwitchMode(NavigationMode),
    /// Perform an accessible action on a DOM node (e.g. click, invoke, toggle).
    PerformAction {
        node_id: NodeId,
        action: AccessibleAction,
    },
    /// The keystroke should pass directly to the browser / web content.
    PassThrough,
}

/// Web controller coordinating active web documents and virtual reading.
pub struct WebController {
    pub buffer: VirtualBuffer,
}

impl WebController {
    /// Creates a new WebController for a given virtual buffer.
    pub fn new(buffer: VirtualBuffer) -> Self {
        Self { buffer }
    }

    /// Returns the current navigation mode (Browse vs Focus).
    pub fn mode(&self) -> NavigationMode {
        self.buffer.mode
    }

    /// Sets the navigation mode explicitly.
    pub fn set_mode(&mut self, mode: NavigationMode) -> Vec<WebAction> {
        if self.buffer.mode == mode {
            return Vec::new();
        }
        self.buffer.mode = mode;
        let mut actions = Vec::new();
        match mode {
            NavigationMode::Browse => {
                actions.push(WebAction::PlaySound("browse_mode.wav"));
                actions.push(WebAction::Speak("Browse mode".to_string()));
            }
            NavigationMode::Focus => {
                actions.push(WebAction::PlaySound("focus_mode.wav"));
                actions.push(WebAction::Speak("Focus mode".to_string()));
            }
        }
        actions.push(WebAction::SwitchMode(mode));
        actions
    }

    /// Manually toggles between Browse Mode and Focus Mode.
    pub fn toggle_mode(&mut self) -> Vec<WebAction> {
        let new_mode = self.buffer.mode.toggle();
        self.set_mode(new_mode)
    }

    /// Handles keyboard input events routed from the engine coordinator.
    pub fn handle_key(&mut self, key: &KeyEvent) -> Vec<WebAction> {
        let mut actions = Vec::new();

        // Universal Escape Hatch: Escape always returns to Browse Mode
        if key.key == Key::Escape && key.modifiers.is_empty() {
            if self.buffer.mode == NavigationMode::Focus {
                return self.set_mode(NavigationMode::Browse);
            }
            return actions;
        }

        // If in Focus Mode, let all keystrokes pass directly to web content
        if self.buffer.mode == NavigationMode::Focus {
            actions.push(WebAction::PassThrough);
            return actions;
        }

        // --- Browse Mode Navigation ---

        // Line-by-line reading: Up Arrow / Down Arrow
        if key.modifiers.is_empty() {
            match key.key {
                Key::DownArrow => {
                    if let Some(line) = self.buffer.next_line() {
                        let text = line.spoken_text();
                        if !text.is_empty() {
                            actions.push(WebAction::Speak(text));
                        }
                    } else {
                        actions.push(WebAction::PlaySound("boundary.wav"));
                    }
                    return actions;
                }
                Key::UpArrow => {
                    if let Some(line) = self.buffer.prev_line() {
                        let text = line.spoken_text();
                        if !text.is_empty() {
                            actions.push(WebAction::Speak(text));
                        }
                    } else {
                        actions.push(WebAction::PlaySound("boundary.wav"));
                    }
                    return actions;
                }
                Key::RightArrow => {
                    if let Some(c) = self.buffer.next_character() {
                        actions.push(WebAction::Speak(c.to_string()));
                    }
                    return actions;
                }
                Key::LeftArrow => {
                    if let Some(c) = self.buffer.prev_character() {
                        actions.push(WebAction::Speak(c.to_string()));
                    }
                    return actions;
                }
                Key::Home => {
                    self.buffer.line_start();
                    if let Some(c) = self.buffer.current_character() {
                        actions.push(WebAction::Speak(c.to_string()));
                    }
                    return actions;
                }
                Key::End => {
                    self.buffer.line_end();
                    if let Some(c) = self.buffer.current_character() {
                        actions.push(WebAction::Speak(c.to_string()));
                    }
                    return actions;
                }
                Key::Enter | Key::Space => {
                    // Activate element or switch to Focus Mode
                    if let Some(run) = self.buffer.current_run() {
                        match run.role {
                            Role::Link => {
                                actions.push(WebAction::PerformAction {
                                    node_id: run.node_id,
                                    action: AccessibleAction::Click,
                                });
                                return actions;
                            }
                            Role::Button | Role::SplitButton => {
                                actions.push(WebAction::PerformAction {
                                    node_id: run.node_id,
                                    action: AccessibleAction::Click,
                                });
                                return actions;
                            }
                            Role::CheckBox => {
                                actions.push(WebAction::PerformAction {
                                    node_id: run.node_id,
                                    action: AccessibleAction::Click,
                                });
                                return actions;
                            }
                            Role::RadioButton => {
                                actions.push(WebAction::PerformAction {
                                    node_id: run.node_id,
                                    action: AccessibleAction::Select,
                                });
                                return actions;
                            }
                            Role::EditableText | Role::ComboBox => {
                                // Switch to Focus Mode for typing
                                let mut mode_actions = self.set_mode(NavigationMode::Focus);
                                actions.append(&mut mode_actions);
                                return actions;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        // Word navigation: Ctrl + Left / Right Arrow
        if key.modifiers == KeyModifiers::CONTROL {
            match key.key {
                Key::RightArrow => {
                    if let Some(word) = self.buffer.next_word() {
                        actions.push(WebAction::Speak(word));
                    }
                    return actions;
                }
                Key::LeftArrow => {
                    if let Some(word) = self.buffer.prev_word() {
                        actions.push(WebAction::Speak(word));
                    }
                    return actions;
                }
                Key::Home => {
                    self.buffer.doc_start();
                    if let Some(line) = self.buffer.current_line() {
                        actions.push(WebAction::Speak(line.spoken_text()));
                    }
                    return actions;
                }
                Key::End => {
                    self.buffer.doc_end();
                    if let Some(line) = self.buffer.current_line() {
                        actions.push(WebAction::Speak(line.spoken_text()));
                    }
                    return actions;
                }
                _ => {}
            }
        }

        // Single-Letter Quick Navigation (H, K, F, B, etc. and Shift + Key)
        let is_pure = key.modifiers.is_empty();
        let is_shift = key.modifiers == KeyModifiers::SHIFT;
        if is_pure || is_shift {
            if let Some((target, is_reverse)) = QuickNavKey::from_key(key.key, is_shift) {
                let target_idx = if is_reverse {
                    QuickNav::find_prev(&self.buffer, target)
                } else {
                    QuickNav::find_next(&self.buffer, target)
                };

                if let Some(idx) = target_idx {
                    self.buffer.set_cursor_line(idx);
                    if let Some(line) = self.buffer.current_line() {
                        actions.push(WebAction::Speak(line.spoken_text()));
                    }
                } else {
                    let dir_str = if is_reverse { "previous" } else { "next" };
                    actions.push(WebAction::Speak(format!("No {} {}", dir_str, target.display_name())));
                }
                return actions;
            }
        }

        actions
    }

    /// Evaluates focus changes from the platform to trigger automatic state transitions.
    /// Non-editable controls and document reading elements default to Browse Mode.
    /// Editable text and inputs switch to Focus Mode.
    pub fn handle_focus_change(&mut self, node: &AccessibleNode) -> Vec<WebAction> {
        let mut actions = Vec::new();

        let should_focus = (node.states.contains(bit_sr_core::states::State::EDITABLE)
            && !node.states.contains(bit_sr_core::states::State::READONLY))
            || matches!(
                node.role,
                Role::EditableText | Role::ComboBox | Role::Slider | Role::SpinButton
            );

        if should_focus {
            if self.buffer.mode != NavigationMode::Focus {
                let mut switch_actions = self.set_mode(NavigationMode::Focus);
                actions.append(&mut switch_actions);
            }
        } else {
            // Non-editable controls (Buttons, Links, CheckBoxes, Headings, Document, etc.) default to Browse Mode
            if self.buffer.mode != NavigationMode::Browse {
                let mut switch_actions = self.set_mode(NavigationMode::Browse);
                actions.append(&mut switch_actions);
            }
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::{BufferLine, TextRun};

    #[test]
    fn test_web_controller_mode_toggling() {
        let buf = VirtualBuffer::default();
        let mut controller = WebController::new(buf);
        assert_eq!(controller.mode(), NavigationMode::Browse);

        let actions = controller.toggle_mode();
        assert_eq!(controller.mode(), NavigationMode::Focus);
        assert!(actions.contains(&WebAction::SwitchMode(NavigationMode::Focus)));

        let actions2 = controller.toggle_mode();
        assert_eq!(controller.mode(), NavigationMode::Browse);
        assert!(actions2.contains(&WebAction::SwitchMode(NavigationMode::Browse)));
    }

    #[test]
    fn test_web_controller_arrow_navigation() {
        let lines = vec![
            BufferLine::new(0, vec![TextRun::new_heading(NodeId(1), "Welcome", 1)]),
            BufferLine::new(1, vec![TextRun::new_text(NodeId(2), "Description text.")]),
        ];
        let buf = VirtualBuffer::with_lines(NodeId(0), lines);
        let mut controller = WebController::new(buf);

        let down_event = KeyEvent::new(Key::DownArrow, bit_sr_core::input::KeyAction::Down, KeyModifiers::empty());

        let actions = controller.handle_key(&down_event);
        assert_eq!(actions, vec![WebAction::Speak("Description text.".to_string())]);
        assert_eq!(controller.buffer.cursor_line, 1);
    }

    #[test]
    fn test_web_controller_quick_nav_heading() {
        let lines = vec![
            BufferLine::new(0, vec![TextRun::new_text(NodeId(1), "Top text.")]),
            BufferLine::new(1, vec![TextRun::new_heading(NodeId(2), "Section 1", 2)]),
        ];
        let buf = VirtualBuffer::with_lines(NodeId(0), lines);
        let mut controller = WebController::new(buf);

        let h_event = KeyEvent::new(Key::H, bit_sr_core::input::KeyAction::Down, KeyModifiers::empty());

        let actions = controller.handle_key(&h_event);
        assert_eq!(actions, vec![WebAction::Speak("heading level 2 Section 1".to_string())]);
        assert_eq!(controller.buffer.cursor_line, 1);
    }

    #[test]
    fn test_escape_returns_to_browse_mode() {
        let mut controller = WebController::new(VirtualBuffer::default());
        controller.set_mode(NavigationMode::Focus);
        assert_eq!(controller.mode(), NavigationMode::Focus);

        let esc_event = KeyEvent::new(Key::Escape, bit_sr_core::input::KeyAction::Down, KeyModifiers::empty());

        let actions = controller.handle_key(&esc_event);
        assert_eq!(controller.mode(), NavigationMode::Browse);
        assert!(actions.contains(&WebAction::SwitchMode(NavigationMode::Browse)));
    }

    #[test]
    fn test_handle_focus_change_auto_mode_switching() {
        let mut controller = WebController::new(VirtualBuffer::default());
        assert_eq!(controller.mode(), NavigationMode::Browse);

        // Focus on a button -> stays in Browse mode
        let mut btn = AccessibleNode::default();
        btn.role = Role::Button;
        let actions = controller.handle_focus_change(&btn);
        assert!(actions.is_empty());
        assert_eq!(controller.mode(), NavigationMode::Browse);

        // Focus on an edit box -> switches to Focus mode
        let mut edit = AccessibleNode::default();
        edit.role = Role::EditableText;
        let actions = controller.handle_focus_change(&edit);
        assert_eq!(controller.mode(), NavigationMode::Focus);
        assert!(actions.contains(&WebAction::SwitchMode(NavigationMode::Focus)));

        // Focus on a link -> switches back to Browse mode
        let mut link = AccessibleNode::default();
        link.role = Role::Link;
        let actions = controller.handle_focus_change(&link);
        assert_eq!(controller.mode(), NavigationMode::Browse);
        assert!(actions.contains(&WebAction::SwitchMode(NavigationMode::Browse)));

        // Focus on a combobox -> switches to Focus mode
        let mut combo = AccessibleNode::default();
        combo.role = Role::ComboBox;
        let actions = controller.handle_focus_change(&combo);
        assert_eq!(controller.mode(), NavigationMode::Focus);
        assert!(actions.contains(&WebAction::SwitchMode(NavigationMode::Focus)));

        // Focus on Document container -> switches back to Browse mode
        let mut doc = AccessibleNode::default();
        doc.role = Role::Document;
        let actions = controller.handle_focus_change(&doc);
        assert_eq!(controller.mode(), NavigationMode::Browse);
        assert!(actions.contains(&WebAction::SwitchMode(NavigationMode::Browse)));
    }
}
