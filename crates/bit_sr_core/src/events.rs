//! Unified Accessibility and System Event Types.

use crate::input::KeyEvent;
use crate::node::AccessibleNode;
use crate::states::State;

#[derive(Debug, Clone, PartialEq)]
pub enum AccessibilityEvent {
    /// Keyboard focus shifted to a new accessible node.
    Focus(AccessibleNode),

    /// A node's state bitflag transitioned.
    StateChange {
        node: AccessibleNode,
        state: State,
        is_set: bool,
    },

    /// An accessible object's name/label was updated.
    NameChange {
        node: AccessibleNode,
        new_name: Option<String>,
    },

    /// A control value (e.g. slider, progress, editable field) changed.
    ValueChange {
        node: AccessibleNode,
        new_value: Option<String>,
    },

    /// Item selected within a list, table, or tab control.
    Selection(AccessibleNode),

    /// Caret insertion offset changed in a text or terminal control.
    CaretMoved {
        node: AccessibleNode,
        offset: i32,
    },

    /// System alert, shell notification, or snap layout result.
    Notification {
        activity_id: String,
        display_string: String,
    },

    /// Top-level application window became active.
    WindowActivated(AccessibleNode),

    /// Top-level application window became inactive.
    WindowDeactivated(AccessibleNode),

    /// Low-level keyboard input event.
    Input(KeyEvent),

    /// Caps Lock hardware state was toggled via double-tap.
    CapsLockToggled(bool),

    /// Instant speech cancellation requested by keypress.
    SpeechInterrupt,

    /// Screen reader action triggered from a menu or tray.
    MenuAction(crate::menu::MenuAction),
}
