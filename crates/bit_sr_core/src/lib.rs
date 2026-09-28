//! Core data models, roles, states, actions, and accessibility tree for bit_sr.

pub mod actions;
pub mod events;
pub mod input;
pub mod node;
pub mod roles;
pub mod states;
pub mod tree;

pub use actions::{AccessibleAction, ActionCapabilities, ActionError, ActionPerformer};
pub use events::AccessibilityEvent;
pub use input::{KeyAction, KeyEvent, KeyModifiers, MouseAction, MouseEvent};
pub use node::{
    AccessibleNode, CollectionInfo, CollectionItemInfo, LiveRegion, NodeId, PositionInfo,
    RangeInfo, Rect, TextSelection,
};
pub use roles::Role;
pub use states::State;
pub use tree::{AccessibilityTree, NavDirection, NavFilter};
