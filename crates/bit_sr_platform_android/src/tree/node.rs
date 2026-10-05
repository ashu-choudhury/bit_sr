//! Accessible node data structures for Android.

use bit_sr_core::node::{NodeId, Rect};
use bit_sr_core::roles::Role;
use bit_sr_core::states::State;

/// Cached representation of an Android accessible element.
#[derive(Debug, Clone, PartialEq)]
pub struct CachedNode {
    /// Unique programmatic identifier (from AOSP source hash or Virtual ID).
    pub id: NodeId,
    /// Unified screen reader role mapped from View class name.
    pub role: Role,
    /// Absolute screen coordinates and dimensions in physical pixels.
    pub bounds: Rect,
    /// Accessibility text label or content description.
    pub label: String,
    /// Active state flags (focused, checked, selected, disabled, etc.).
    pub states: State,
}

impl CachedNode {
    /// Creates a new cached node.
    pub fn new(id: NodeId, role: Role, bounds: Rect, label: String, states: State) -> Self {
        Self {
            id,
            role,
            bounds,
            label,
            states,
        }
    }

    /// Whether this node has a specific accessibility state.
    pub fn has_state(&self, state: State) -> bool {
        self.states.contains(state)
    }
}
