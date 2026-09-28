//! Accessible Node, Hierarchy Relations, and Spatial Geometry Structures.
//! Modeled after Google Android's `AccessibilityNodeInfo` hierarchy and semantics.

use crate::actions::{AccessibleAction, ActionCapabilities};
use crate::roles::Role;
use crate::states::State;

/// Unique 64-bit identifier for an accessible node within the tree session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct NodeId(pub u64);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:X}", self.0)
    }
}

/// Screen coordinate rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn right(&self) -> f64 {
        self.left + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.top + self.height
    }

    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        x >= self.left && x <= self.right() && y >= self.top && y <= self.bottom()
    }
}

/// Numerical range information (sliders, progress bars, scrollbars).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RangeInfo {
    pub min: f64,
    pub max: f64,
    pub current: f64,
    pub step: f64,
}

/// Collection metadata (tables, grids, lists).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CollectionInfo {
    pub row_count: usize,
    pub column_count: usize,
    pub is_hierarchical: bool,
}

/// Individual item metadata within a collection (cells, list items, tree items).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CollectionItemInfo {
    pub row_index: usize,
    pub column_index: usize,
    pub row_span: usize,
    pub column_span: usize,
    pub is_heading: bool,
    pub is_selected: bool,
}

/// Text caret or selection range within an editable text control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextSelection {
    pub start: usize,
    pub end: usize,
}

/// ARIA / Accessibility live region update urgency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LiveRegion {
    #[default]
    None,
    Polite,
    Assertive,
}

/// Hierarchical position metadata within a group or list (e.g. "Item 3 of 12, level 2").
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PositionInfo {
    pub position_in_set: Option<i32>,
    pub size_of_set: Option<i32>,
    pub row_index: Option<i32>,
    pub column_index: Option<i32>,
    pub level: Option<i32>,
}

/// Comprehensive platform-agnostic accessible node.
/// Contains complete tree relations (parent, children, labeled_by),
/// semantic attributes, bounds, and executable actions.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AccessibleNode {
    /// Unique identifier for this node.
    pub id: NodeId,

    /// Semantic role (e.g. Button, CheckBox, ListItem, Heading).
    pub role: Role,

    /// State bitflags (Focused, Selected, Expanded, Checked, Disabled, etc.).
    pub states: State,

    /// Accessible name / label.
    pub name: Option<String>,

    /// Value / current text (e.g. text in an edit field, percentage on a slider).
    pub value: Option<String>,

    /// Accessible description / secondary explanatory text.
    pub description: Option<String>,

    /// Hint text explaining interaction (e.g. "Double tap to activate").
    pub hint: Option<String>,

    /// Keyboard accelerator or shortcut (e.g. "Ctrl+O", "Alt+F").
    pub keyboard_shortcut: Option<String>,

    /// Native platform class name (e.g. "DirectUIHWND", "SysListView32", "GtkButton").
    pub class_name: Option<String>,

    /// Stable automation identifier (e.g. "SearchEditBox", "SubmitButton").
    pub automation_id: Option<String>,

    /// Physical screen bounding box.
    pub bounds: Option<Rect>,

    /// Relative bounding box within the parent node.
    pub bounds_in_parent: Option<Rect>,

    // --- Tree Relations (Android-inspired) ---

    /// Node ID of the parent in the accessibility tree.
    pub parent: Option<NodeId>,

    /// Ordered list of child node IDs.
    pub children: Vec<NodeId>,

    /// Node ID of the element that serves as this node's label (if distinct).
    pub labeled_by: Option<NodeId>,

    /// Natural reading/traversal order predecessor hint.
    pub traversal_before: Option<NodeId>,

    /// Natural reading/traversal order successor hint.
    pub traversal_after: Option<NodeId>,

    // --- Rich Semantic Attributes ---

    /// Position within group or set (e.g., 3 of 15).
    pub position_info: PositionInfo,

    /// Range info for sliders, progress bars, etc.
    pub range_info: Option<RangeInfo>,

    /// Collection info if this node represents a table, grid, or list.
    pub collection_info: Option<CollectionInfo>,

    /// Collection item info if this node is inside a table, grid, or list.
    pub collection_item_info: Option<CollectionItemInfo>,

    /// Text selection offsets if this node contains editable or selectable text.
    pub text_selection: Option<TextSelection>,

    /// Live region mode for automatic asynchronous announcements.
    pub live_region: LiveRegion,

    /// Process ID hosting this accessible node.
    pub process_id: Option<u32>,

    /// Supported actions executable on this node.
    pub actions: Vec<AccessibleAction>,

    /// Fast bitflags of supported capabilities.
    pub action_capabilities: ActionCapabilities,
}

impl AccessibleNode {
    /// Creates a new accessible node with given ID and Role.
    pub fn new(id: NodeId, role: Role) -> Self {
        Self {
            id,
            role,
            ..Default::default()
        }
    }

    /// Adds a child node ID to this node's children list.
    pub fn add_child(&mut self, child_id: NodeId) {
        if !self.children.contains(&child_id) {
            self.children.push(child_id);
        }
    }

    /// Sets the parent node ID.
    pub fn set_parent(&mut self, parent_id: Option<NodeId>) {
        self.parent = parent_id;
    }

    /// Adds an executable action to this node and updates its action capabilities.
    pub fn add_action(&mut self, action: AccessibleAction) {
        if let Some(cap) = action.capability() {
            self.action_capabilities |= cap;
        }
        if !self.actions.contains(&action) {
            self.actions.push(action);
        }
    }

    // --- Quick Semantic Predicates ---

    pub fn is_focusable(&self) -> bool {
        self.states.contains(State::FOCUSABLE)
    }

    pub fn is_focused(&self) -> bool {
        self.states.contains(State::FOCUSED)
    }

    pub fn is_selected(&self) -> bool {
        self.states.contains(State::SELECTED)
    }

    pub fn is_checkable(&self) -> bool {
        self.states.contains(State::CHECKABLE) || matches!(self.role, Role::CheckBox | Role::RadioButton | Role::Switch)
    }

    pub fn is_checked(&self) -> bool {
        self.states.contains(State::CHECKED)
    }

    pub fn is_expanded(&self) -> bool {
        self.states.contains(State::EXPANDED)
    }

    pub fn is_collapsed(&self) -> bool {
        self.states.contains(State::COLLAPSED)
    }

    pub fn is_disabled(&self) -> bool {
        self.states.contains(State::UNAVAILABLE)
    }

    pub fn is_heading(&self) -> bool {
        self.role == Role::Heading || self.collection_item_info.map_or(false, |info| info.is_heading)
    }

    pub fn is_clickable(&self) -> bool {
        self.action_capabilities.contains(ActionCapabilities::CLICK)
            || matches!(self.role, Role::Button | Role::Link | Role::CheckBox | Role::RadioButton | Role::MenuItem)
    }

    pub fn is_editable(&self) -> bool {
        matches!(self.role, Role::EditableText | Role::Document | Role::Terminal)
            || (!self.states.contains(State::READONLY) && self.action_capabilities.contains(ActionCapabilities::SET_TEXT))
            || self.states.contains(State::EDITABLE)
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}
