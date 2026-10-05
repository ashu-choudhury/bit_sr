//! Bounding box spatial tree and memory-efficient node cache for Android.
//! Prevents IPC and Binder round-trips while dragging fingers across the display.

use crate::tree::node::CachedNode;

/// In-memory spatial index of on-screen elements for instant hit testing.
#[derive(Debug, Default)]
pub struct SpatialNodeCache {
    nodes: Vec<CachedNode>,
    active_window_id: i32,
}

impl SpatialNodeCache {
    /// Creates a new empty spatial cache.
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(128),
            active_window_id: 0,
        }
    }

    /// Replaces the active node tree for a window with harvested nodes.
    pub fn update_window_tree(&mut self, window_id: i32, nodes: Vec<CachedNode>) {
        self.active_window_id = window_id;
        self.nodes = nodes;
    }

    /// Performs sub-microsecond hit testing for raw screen coordinates (x, y).
    /// Returns the smallest leaf node enclosing the given point.
    pub fn hit_test(&self, x: f64, y: f64) -> Option<&CachedNode> {
        // Iterate backwards (front-to-back z-order) and find smallest enclosing rectangle
        let mut best_match: Option<&CachedNode> = None;
        let mut min_area = f64::MAX;

        for node in self.nodes.iter().rev() {
            if node.bounds.contains_point(x, y) {
                let area = node.bounds.width * node.bounds.height;
                if area > 0.0 && area < min_area {
                    min_area = area;
                    best_match = Some(node);
                }
            }
        }

        best_match
    }

    /// Finds node by its ID.
    pub fn get_by_id(&self, id: u64) -> Option<&CachedNode> {
        self.nodes.iter().find(|n| n.id.0 == id)
    }

    /// Returns the next node in linear reading order relative to the given node ID.
    pub fn next_node(&self, current_id: u64) -> Option<&CachedNode> {
        if self.nodes.is_empty() {
            return None;
        }

        let current_idx = self.nodes.iter().position(|n| n.id.0 == current_id);
        match current_idx {
            Some(idx) => {
                if idx + 1 < self.nodes.len() {
                    Some(&self.nodes[idx + 1])
                } else {
                    None // Reached boundary
                }
            }
            None => self.nodes.first(),
        }
    }

    /// Returns the previous node in linear reading order relative to the given node ID.
    pub fn previous_node(&self, current_id: u64) -> Option<&CachedNode> {
        if self.nodes.is_empty() {
            return None;
        }

        let current_idx = self.nodes.iter().position(|n| n.id.0 == current_id);
        match current_idx {
            Some(idx) => {
                if idx > 0 {
                    Some(&self.nodes[idx - 1])
                } else {
                    None // Reached boundary
                }
            }
            None => self.nodes.last(),
        }
    }

    /// Returns all cached nodes.
    pub fn all_nodes(&self) -> &[CachedNode] {
        &self.nodes
    }

    /// Active window ID.
    pub fn active_window_id(&self) -> i32 {
        self.active_window_id
    }

    /// Total number of cached nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the cache is currently empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Clears the cache.
    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit_sr_core::node::{NodeId, Rect};
    use bit_sr_core::roles::Role;
    use bit_sr_core::states::State;

    #[test]
    fn test_spatial_hit_testing() {
        let mut cache = SpatialNodeCache::new();

        let container = CachedNode {
            id: NodeId(1),
            role: Role::Window,
            bounds: Rect {
                left: 0.0,
                top: 0.0,
                width: 1080.0,
                height: 2400.0,
            },
            label: "Main Window".to_string(),
            states: State::empty(),
        };

        let button = CachedNode {
            id: NodeId(2),
            role: Role::Button,
            bounds: Rect {
                left: 100.0,
                top: 200.0,
                width: 300.0,
                height: 120.0,
            },
            label: "Submit".to_string(),
            states: State::empty(),
        };

        cache.update_window_tree(1, vec![container, button]);

        // Point inside the button
        let hit = cache.hit_test(150.0, 250.0);
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().id, NodeId(2));
        assert_eq!(hit.unwrap().label, "Submit");

        // Point outside the button but inside container
        let hit_container = cache.hit_test(50.0, 50.0);
        assert!(hit_container.is_some());
        assert_eq!(hit_container.unwrap().id, NodeId(1));

        // Point completely outside
        let hit_outside = cache.hit_test(1200.0, 2500.0);
        assert!(hit_outside.is_none());
    }

    #[test]
    fn test_linear_navigation() {
        let mut cache = SpatialNodeCache::new();
        let n1 = CachedNode {
            id: NodeId(10),
            role: Role::StaticText,
            bounds: Rect::default(),
            label: "Header".to_string(),
            states: State::empty(),
        };
        let n2 = CachedNode {
            id: NodeId(20),
            role: Role::Button,
            bounds: Rect::default(),
            label: "OK".to_string(),
            states: State::empty(),
        };

        cache.update_window_tree(1, vec![n1, n2]);

        assert_eq!(cache.next_node(10).unwrap().id, NodeId(20));
        assert!(cache.next_node(20).is_none());
        assert_eq!(cache.previous_node(20).unwrap().id, NodeId(10));
        assert!(cache.previous_node(10).is_none());
    }
}
