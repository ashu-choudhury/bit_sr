//! Focus & Context Tracker: Maintains active focus state, window context, and deduplication.

use bit_sr_core::node::AccessibleNode;

/// Result of evaluating an incoming focus event against previous state.
#[derive(Debug, Clone, PartialEq)]
pub enum FocusTransition {
    /// Identical to previous focus; should be debounced / suppressed.
    Redundant,
    /// Switched to an element in a different window or application.
    NewWindow { window_title: Option<String> },
    /// Moved to a new element within the same window.
    NewElement,
}

pub struct FocusTracker {
    current_focus: Option<AccessibleNode>,
    current_window_title: Option<String>,
    current_process_id: Option<u32>,
}

impl FocusTracker {
    pub fn new() -> Self {
        Self {
            current_focus: None,
            current_window_title: None,
            current_process_id: None,
        }
    }

    /// Evaluates an incoming focus event, updates internal state, and returns the transition type.
    pub fn on_focus(&mut self, node: AccessibleNode) -> (FocusTransition, &AccessibleNode) {
        let is_redundant = if let Some(prev) = &self.current_focus {
            prev.id == node.id && prev.name == node.name && prev.value == node.value && prev.states == node.states
        } else {
            false
        };

        if is_redundant {
            return (FocusTransition::Redundant, self.current_focus.as_ref().unwrap());
        }

        let window_changed = if let (Some(prev_pid), Some(new_pid)) = (self.current_process_id, node.process_id) {
            prev_pid != new_pid
        } else {
            false
        };

        self.current_process_id = node.process_id;
        self.current_focus = Some(node);

        if window_changed {
            (
                FocusTransition::NewWindow {
                    window_title: self.current_window_title.clone(),
                },
                self.current_focus.as_ref().unwrap(),
            )
        } else {
            (FocusTransition::NewElement, self.current_focus.as_ref().unwrap())
        }
    }

    /// Updates active window title and process ID.
    pub fn on_window_activated(&mut self, window_node: &AccessibleNode) -> Option<String> {
        let title = window_node.name.clone();
        self.current_window_title = title.clone();
        if let Some(pid) = window_node.process_id {
            self.current_process_id = Some(pid);
        }
        title
    }

    /// Explicitly updates the current window title.
    pub fn set_window_title(&mut self, title: String) {
        self.current_window_title = Some(title);
    }

    /// Returns a reference to the currently focused node.
    pub fn current_focus(&self) -> Option<&AccessibleNode> {
        self.current_focus.as_ref()
    }

    /// Returns the active window title.
    pub fn current_window_title(&self) -> Option<&str> {
        self.current_window_title.as_deref()
    }

    /// Clears tracking state.
    pub fn clear(&mut self) {
        self.current_focus = None;
        self.current_window_title = None;
        self.current_process_id = None;
    }
}

impl Default for FocusTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit_sr_core::node::NodeId;

    #[test]
    fn test_focus_deduplication() {
        let mut tracker = FocusTracker::new();
        let node1 = AccessibleNode {
            id: NodeId(10),
            name: Some("Item A".to_string()),
            process_id: Some(1234),
            ..Default::default()
        };

        let (t1, _) = tracker.on_focus(node1.clone());
        assert_eq!(t1, FocusTransition::NewElement);

        // Immediate identical event -> Redundant
        let (t2, _) = tracker.on_focus(node1);
        assert_eq!(t2, FocusTransition::Redundant);

        // Movement to new item
        let node2 = AccessibleNode {
            id: NodeId(11),
            name: Some("Item B".to_string()),
            process_id: Some(1234),
            ..Default::default()
        };
        let (t3, _) = tracker.on_focus(node2);
        assert_eq!(t3, FocusTransition::NewElement);
    }
}
