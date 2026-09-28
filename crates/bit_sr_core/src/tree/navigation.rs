//! Navigation Directions and Filtering Predicates for Tree Traversal.

use crate::node::AccessibleNode;
use crate::roles::Role;
use std::sync::Arc;

/// Navigation direction for structural traversal across the accessibility tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavDirection {
    /// Move forward in document/reading order (Depth-First Search pre-order).
    NextInOrder,

    /// Move backward in document/reading order (Reverse DFS).
    PreviousInOrder,

    /// Move up to the direct parent container.
    Parent,

    /// Move down to the first child element.
    FirstChild,

    /// Move down to the last child element.
    LastChild,

    /// Move to the next sibling within the same parent.
    NextSibling,

    /// Move to the previous sibling within the same parent.
    PreviousSibling,
}

/// Filter criteria determining which nodes are eligible during navigation.
#[derive(Clone)]
pub enum NavFilter {
    /// Accept every node in the hierarchy.
    All,

    /// Only accept nodes that can receive keyboard or accessibility focus.
    Focusable,

    /// Only accept nodes that have a specific role (e.g. Button, ListItem, Link).
    Role(Role),

    /// Only accept heading elements (for quick document navigation).
    Heading,

    /// Only accept interactive elements (clickable, checkable, selectable, expandable).
    Interactive,

    /// Only accept editable text elements.
    Editable,

    /// Only accept structural landmark regions (banner, main, navigation, etc.).
    Landmark,

    /// Custom user-defined or extension-defined predicate.
    Custom(Arc<dyn Fn(&AccessibleNode) -> bool + Send + Sync>),
}

impl std::fmt::Debug for NavFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "NavFilter::All"),
            Self::Focusable => write!(f, "NavFilter::Focusable"),
            Self::Role(r) => write!(f, "NavFilter::Role({:?})", r),
            Self::Heading => write!(f, "NavFilter::Heading"),
            Self::Interactive => write!(f, "NavFilter::Interactive"),
            Self::Editable => write!(f, "NavFilter::Editable"),
            Self::Landmark => write!(f, "NavFilter::Landmark"),
            Self::Custom(_) => write!(f, "NavFilter::Custom(<closure>)"),
        }
    }
}

impl NavFilter {
    /// Evaluates whether the given node satisfies this navigation filter.
    pub fn matches(&self, node: &AccessibleNode) -> bool {
        match self {
            Self::All => true,
            Self::Focusable => node.is_focusable(),
            Self::Role(expected) => node.role == *expected,
            Self::Heading => node.is_heading(),
            Self::Interactive => {
                node.is_clickable()
                    || node.is_focusable()
                    || node.is_checkable()
                    || node.is_selected()
                    || node.is_expanded()
                    || node.is_collapsed()
                    || node.is_editable()
            }
            Self::Editable => node.is_editable(),
            Self::Landmark => matches!(
                node.role,
                Role::Landmark | Role::Document | Role::Application | Role::Grouping
            ),
            Self::Custom(predicate) => (predicate)(node),
        }
    }
}
