#![forbid(unsafe_code)]
//! Core data models, roles, states, actions, and accessibility tree for bit_sr.

pub mod actions;
pub mod events;
pub mod i18n;
pub mod input;
pub mod menu;
pub mod node;
pub mod roles;
pub mod states;
pub mod text;
pub mod tree;

pub use actions::{AccessibleAction, ActionCapabilities, ActionError, ActionPerformer};
pub use events::AccessibilityEvent;
pub use i18n::{LocaleInfo, LocalizationManager};
pub use input::{
    GestureParseError, InputGesture, Key, KeyAction, KeyEvent, KeyModifiers, MouseAction,
    MouseEvent, SRKeyAction, SRKeyConfig, SRModifierTracker,
};
pub use menu::{MenuAction, MenuItemData};
pub use node::{
    AccessibleNode, CollectionInfo, CollectionItemInfo, LiveRegion, NodeId, PositionInfo,
    RangeInfo, Rect, TextSelection,
};
pub use roles::Role;
pub use states::State;
pub use text::{TextProvider, TextUnit};
pub use tree::{AccessibilityTree, NavDirection, NavFilter};

