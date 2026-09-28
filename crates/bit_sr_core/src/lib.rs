//! Core data models, roles, states, and event definitions for bit_sr.

pub mod events;
pub mod input;
pub mod node;
pub mod roles;
pub mod states;

pub use events::AccessibilityEvent;
pub use input::{KeyAction, KeyEvent, KeyModifiers, MouseAction, MouseEvent};
pub use node::{AccessibleNode, NodeId, PositionInfo, Rect};
pub use roles::Role;
pub use states::State;
