//! Bounding box spatial tree and memory-efficient node cache for Android.
//! Prevents IPC and Binder round-trips while dragging fingers across the display.

use bit_sr_core::node::{NodeId, Rect};
use bit_sr_core::roles::Role;
use bit_sr_core::states::State;

/// Cached representation of an Android accessible element.
#[derive(Debug, Clone)]
pub struct CachedNode {
    pub id: NodeId,
    pub role: Role,
    pub bounds: Rect,
    pub label: String,
    pub states: Vec<State>,
}

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

    #[test]
    fn test_spatial_hit_testing() {
        let mut cache = SpatialNodeCache::new();
        
        let container = CachedNode {
            id: NodeId(1),
            role: Role::Window,
            bounds: Rect { left: 0.0, top: 0.0, width: 1080.0, height: 2400.0 },
            label: "Main Window".to_string(),
            states: vec![],
        };

        let button = CachedNode {
            id: NodeId(2),
            role: Role::Button,
            bounds: Rect { left: 100.0, top: 200.0, width: 300.0, height: 120.0 },
            label: "Submit".to_string(),
            states: vec![],
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
}
