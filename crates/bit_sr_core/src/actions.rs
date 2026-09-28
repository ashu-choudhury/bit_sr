//! Android-Inspired Unified Accessibility Actions and Dispatch Protocol.
//! Aligned with modern screen reader interaction models.

use crate::node::NodeId;
use bitflags::bitflags;

bitflags! {
    /// Bitflags representing supported actions for zero-overhead capability checks.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ActionCapabilities: u32 {
        const CLICK           = 1 << 0;
        const LONG_CLICK      = 1 << 1;
        const FOCUS           = 1 << 2;
        const CLEAR_FOCUS     = 1 << 3;
        const SELECT          = 1 << 4;
        const CLEAR_SELECTION = 1 << 5;
        const EXPAND          = 1 << 6;
        const COLLAPSE        = 1 << 7;
        const DISMISS         = 1 << 8;
        const SCROLL_FORWARD  = 1 << 9;
        const SCROLL_BACKWARD = 1 << 10;
        const SCROLL_LEFT     = 1 << 11;
        const SCROLL_RIGHT    = 1 << 12;
        const SET_TEXT        = 1 << 13;
        const SET_SELECTION   = 1 << 14;
        const COPY            = 1 << 15;
        const CUT             = 1 << 16;
        const PASTE           = 1 << 17;
        const SHOW_ON_SCREEN  = 1 << 18;
        const CONTEXT_CLICK   = 1 << 19;
    }
}

/// Unified accessibility action executable on a node.
/// Modeled after Android's `AccessibilityNodeInfo.AccessibilityAction`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AccessibleAction {
    /// Perform the primary click/activation action (e.g. button press, checkbox toggle).
    Click,

    /// Press and hold / secondary action (e.g. context menu).
    LongClick,

    /// Request accessibility or input focus on this node.
    Focus,

    /// Clear focus from this node.
    ClearFocus,

    /// Select this item within its container (list item, tab, radio button).
    Select,

    /// Deselect this item.
    ClearSelection,

    /// Expand a collapsible node (tree node, combo box, dropdown).
    Expand,

    /// Collapse an expanded node.
    Collapse,

    /// Dismiss a transient window, tooltip, dialog, or popup.
    Dismiss,

    /// Scroll content forward or downward.
    ScrollForward,

    /// Scroll content backward or upward.
    ScrollBackward,

    /// Scroll content horizontally left.
    ScrollLeft,

    /// Scroll content horizontally right.
    ScrollRight,

    /// Replace the entire text content of an editable node.
    SetText(String),

    /// Set character selection range [start, end) within an editable node.
    SetSelection { start: usize, end: usize },

    /// Copy current text selection to clipboard.
    Copy,

    /// Cut current text selection to clipboard.
    Cut,

    /// Paste clipboard contents into editable node.
    Paste,

    /// Scroll the viewport so this element becomes visible on screen.
    ShowOnScreen,

    /// Open the context menu for this element.
    ContextClick,

    /// Application-defined custom action with unique ID and localized human-readable label.
    Custom { id: u32, label: String },
}

impl AccessibleAction {
    /// Returns the corresponding bitflag capability, if this is a standard action.
    pub fn capability(&self) -> Option<ActionCapabilities> {
        match self {
            Self::Click => Some(ActionCapabilities::CLICK),
            Self::LongClick => Some(ActionCapabilities::LONG_CLICK),
            Self::Focus => Some(ActionCapabilities::FOCUS),
            Self::ClearFocus => Some(ActionCapabilities::CLEAR_FOCUS),
            Self::Select => Some(ActionCapabilities::SELECT),
            Self::ClearSelection => Some(ActionCapabilities::CLEAR_SELECTION),
            Self::Expand => Some(ActionCapabilities::EXPAND),
            Self::Collapse => Some(ActionCapabilities::COLLAPSE),
            Self::Dismiss => Some(ActionCapabilities::DISMISS),
            Self::ScrollForward => Some(ActionCapabilities::SCROLL_FORWARD),
            Self::ScrollBackward => Some(ActionCapabilities::SCROLL_BACKWARD),
            Self::ScrollLeft => Some(ActionCapabilities::SCROLL_LEFT),
            Self::ScrollRight => Some(ActionCapabilities::SCROLL_RIGHT),
            Self::SetText(_) => Some(ActionCapabilities::SET_TEXT),
            Self::SetSelection { .. } => Some(ActionCapabilities::SET_SELECTION),
            Self::Copy => Some(ActionCapabilities::COPY),
            Self::Cut => Some(ActionCapabilities::CUT),
            Self::Paste => Some(ActionCapabilities::PASTE),
            Self::ShowOnScreen => Some(ActionCapabilities::SHOW_ON_SCREEN),
            Self::ContextClick => Some(ActionCapabilities::CONTEXT_CLICK),
            Self::Custom { .. } => None,
        }
    }

    /// Human-readable default label describing the action for speech output.
    pub fn display_label(&self) -> &str {
        match self {
            Self::Click => "activate",
            Self::LongClick => "long click",
            Self::Focus => "focus",
            Self::ClearFocus => "clear focus",
            Self::Select => "select",
            Self::ClearSelection => "deselect",
            Self::Expand => "expand",
            Self::Collapse => "collapse",
            Self::Dismiss => "dismiss",
            Self::ScrollForward => "scroll forward",
            Self::ScrollBackward => "scroll backward",
            Self::ScrollLeft => "scroll left",
            Self::ScrollRight => "scroll right",
            Self::SetText(_) => "set text",
            Self::SetSelection { .. } => "set selection",
            Self::Copy => "copy",
            Self::Cut => "cut",
            Self::Paste => "paste",
            Self::ShowOnScreen => "show on screen",
            Self::ContextClick => "context menu",
            Self::Custom { label, .. } => label.as_str(),
        }
    }
}

/// Errors that can occur when performing an accessibility action on a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionError {
    /// The target node was not found in the tree.
    NodeNotFound(NodeId),

    /// The requested action is not supported by the target node.
    UnsupportedAction(AccessibleAction),

    /// The target node is disabled or unavailable.
    NodeDisabled,

    /// The target window or application process is unresponsive or hung.
    TargetProcessHung,

    /// Underlying OS platform driver failure.
    PlatformFailure(String),
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeNotFound(id) => write!(f, "Node {:?} not found", id),
            Self::UnsupportedAction(act) => write!(f, "Action {:?} not supported by node", act),
            Self::NodeDisabled => write!(f, "Target node is disabled"),
            Self::TargetProcessHung => write!(f, "Target application process is hung"),
            Self::PlatformFailure(msg) => write!(f, "Platform action error: {}", msg),
        }
    }
}

impl std::error::Error for ActionError {}

/// Trait implemented by platform drivers (Windows UIA/MSAA, Linux AT-SPI2)
/// to physically execute actions requested by the screen reader engine or extensions.
pub trait ActionPerformer: Send + Sync {
    /// Dispatches and executes an action on the specified accessible node.
    fn perform_action(&self, target: NodeId, action: &AccessibleAction) -> Result<(), ActionError>;
}
