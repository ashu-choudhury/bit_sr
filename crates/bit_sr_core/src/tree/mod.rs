//! Platform-Agnostic In-Memory Accessibility Tree.
//! Inspired by Android's hierarchical accessibility model with bidirectional relations,
//! pre-order depth-first reading traversal, and semantic filtering.

pub mod navigation;

pub use navigation::{NavDirection, NavFilter};

use crate::node::{AccessibleNode, NodeId};
use crate::roles::Role;
use std::collections::{HashMap, VecDeque};

/// A complete, platform-agnostic accessibility tree hierarchy.
/// Stores nodes in a flat hash-map keyed by `NodeId`, while maintaining
/// strict bidirectional parent <-> child pointers.
#[derive(Debug, Clone, Default)]
pub struct AccessibilityTree {
    nodes: HashMap<NodeId, AccessibleNode>,
    root_id: Option<NodeId>,
}

impl AccessibilityTree {
    /// Creates a new empty accessibility tree.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root_id: None,
        }
    }

    /// Sets the root node ID of this tree.
    pub fn set_root(&mut self, root_id: NodeId) {
        self.root_id = Some(root_id);
    }

    /// Returns the root node ID, if set.
    pub fn root_id(&self) -> Option<NodeId> {
        self.root_id
    }

    /// Returns a reference to the root node, if it exists in the tree.
    pub fn root(&self) -> Option<&AccessibleNode> {
        self.root_id.and_then(|id| self.nodes.get(&id))
    }

    /// Returns a reference to a node by its unique ID.
    pub fn get(&self, id: NodeId) -> Option<&AccessibleNode> {
        self.nodes.get(&id)
    }

    /// Returns a mutable reference to a node by its unique ID.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut AccessibleNode> {
        self.nodes.get_mut(&id)
    }

    /// Returns true if a node with the given ID exists in the tree.
    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Returns the total number of nodes in the tree.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the tree has no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Clears the tree completely.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.root_id = None;
    }

    // --- Insertion and Linking ---

    /// Inserts or updates a node in the tree.
    /// If the tree is empty and no root is designated, this node becomes the root.
    pub fn insert(&mut self, node: AccessibleNode) {
        let id = node.id;
        if self.root_id.is_none() {
            self.root_id = Some(id);
        }
        self.nodes.insert(id, node);
    }

    /// Inserts a child node and establishes bidirectional parent <-> child links.
    pub fn attach_child(&mut self, parent_id: NodeId, mut child: AccessibleNode) {
        let child_id = child.id;
        child.parent = Some(parent_id);
        self.nodes.insert(child_id, child);

        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.add_child(child_id);
        }
    }

    /// Links an existing child node to an existing parent node.
    pub fn link_parent_child(&mut self, parent_id: NodeId, child_id: NodeId) {
        if parent_id == child_id {
            return; // Prevent self-parenting cycle
        }
        if let Some(child) = self.nodes.get_mut(&child_id) {
            child.parent = Some(parent_id);
        }
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.add_child(child_id);
        }
    }

    // --- Removal ---

    /// Removes a single node from the tree and unlinks it from its parent's children list.
    /// Does not remove child nodes.
    pub fn remove_node(&mut self, id: NodeId) -> Option<AccessibleNode> {
        let node = self.nodes.remove(&id)?;

        // Unlink from parent
        if let Some(parent_id) = node.parent {
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                parent.children.retain(|&c| c != id);
            }
        }

        // If this was root, clear root
        if self.root_id == Some(id) {
            self.root_id = None;
        }

        Some(node)
    }

    /// Recursively removes a node and all of its descendants from the tree.
    pub fn remove_subtree(&mut self, id: NodeId) -> Vec<AccessibleNode> {
        let mut removed = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(id);

        while let Some(current_id) = queue.pop_front() {
            if let Some(node) = self.remove_node(current_id) {
                for &child_id in &node.children {
                    queue.push_back(child_id);
                }
                removed.push(node);
            }
        }

        removed
    }

    // --- Relational Queries (Parent, Children, Siblings) ---

    /// Returns the direct parent of the specified node.
    pub fn parent_of(&self, id: NodeId) -> Option<&AccessibleNode> {
        let node = self.get(id)?;
        let parent_id = node.parent?;
        self.get(parent_id)
    }

    /// Returns an iterator over all direct children of the specified node.
    pub fn children_of(&self, id: NodeId) -> impl Iterator<Item = &AccessibleNode> {
        let child_ids = self.get(id).map(|n| n.children.clone()).unwrap_or_default();
        child_ids.into_iter().filter_map(|cid| self.get(cid))
    }

    /// Returns the first child of the specified node.
    pub fn first_child_of(&self, id: NodeId) -> Option<&AccessibleNode> {
        let node = self.get(id)?;
        let first_id = node.children.first()?;
        self.get(*first_id)
    }

    /// Returns the last child of the specified node.
    pub fn last_child_of(&self, id: NodeId) -> Option<&AccessibleNode> {
        let node = self.get(id)?;
        let last_id = node.children.last()?;
        self.get(*last_id)
    }

    /// Returns the next sibling of the specified node within the parent's children.
    pub fn next_sibling_of(&self, id: NodeId) -> Option<&AccessibleNode> {
        let node = self.get(id)?;
        let parent_id = node.parent?;
        let parent = self.get(parent_id)?;
        let idx = parent.children.iter().position(|&cid| cid == id)?;
        let next_id = parent.children.get(idx + 1)?;
        self.get(*next_id)
    }

    /// Returns the previous sibling of the specified node within the parent's children.
    pub fn previous_sibling_of(&self, id: NodeId) -> Option<&AccessibleNode> {
        let node = self.get(id)?;
        let parent_id = node.parent?;
        let parent = self.get(parent_id)?;
        let idx = parent.children.iter().position(|&cid| cid == id)?;
        if idx == 0 {
            return None;
        }
        let prev_id = parent.children.get(idx - 1)?;
        self.get(*prev_id)
    }

    /// Returns a list of all ancestor nodes ordered from immediate parent up to the root.
    pub fn ancestors_of(&self, id: NodeId) -> Vec<&AccessibleNode> {
        let mut ancestors = Vec::new();
        let mut curr_id = id;
        while let Some(parent) = self.parent_of(curr_id) {
            ancestors.push(parent);
            curr_id = parent.id;
        }
        ancestors
    }

    /// Returns a list of all descendant nodes in Depth-First Search pre-order.
    pub fn descendants_of(&self, id: NodeId) -> Vec<&AccessibleNode> {
        let mut descendants = Vec::new();
        let mut stack = Vec::new();

        // Push children in reverse order so left-most child is popped first
        if let Some(node) = self.get(id) {
            for &child_id in node.children.iter().rev() {
                stack.push(child_id);
            }
        }

        while let Some(curr_id) = stack.pop() {
            if let Some(curr_node) = self.get(curr_id) {
                descendants.push(curr_node);
                for &child_id in curr_node.children.iter().rev() {
                    stack.push(child_id);
                }
            }
        }

        descendants
    }

    // --- Structural Navigation (Linear Reading Order & Directions) ---

    /// Dispatches navigation from a starting node in the specified direction with filtering.
    pub fn navigate(
        &self,
        from: NodeId,
        dir: NavDirection,
        filter: &NavFilter,
    ) -> Option<&AccessibleNode> {
        match dir {
            NavDirection::NextInOrder => self.next_in_order(from, filter),
            NavDirection::PreviousInOrder => self.previous_in_order(from, filter),
            NavDirection::Parent => {
                let mut curr_id = from;
                while let Some(parent) = self.parent_of(curr_id) {
                    if filter.matches(parent) {
                        return Some(parent);
                    }
                    curr_id = parent.id;
                }
                None
            }
            NavDirection::FirstChild => {
                let first = self.first_child_of(from)?;
                if filter.matches(first) {
                    Some(first)
                } else {
                    self.next_in_order(first.id, filter)
                }
            }
            NavDirection::LastChild => {
                let last = self.last_child_of(from)?;
                if filter.matches(last) {
                    Some(last)
                } else {
                    self.previous_in_order(last.id, filter)
                }
            }
            NavDirection::NextSibling => {
                let mut curr_id = from;
                while let Some(next) = self.next_sibling_of(curr_id) {
                    if filter.matches(next) {
                        return Some(next);
                    }
                    curr_id = next.id;
                }
                None
            }
            NavDirection::PreviousSibling => {
                let mut curr_id = from;
                while let Some(prev) = self.previous_sibling_of(curr_id) {
                    if filter.matches(prev) {
                        return Some(prev);
                    }
                    curr_id = prev.id;
                }
                None
            }
        }
    }

    /// Moves forward in document/reading order (pre-order DFS) to the next node matching `filter`.
    pub fn next_in_order(&self, current: NodeId, filter: &NavFilter) -> Option<&AccessibleNode> {
        let mut curr = self.get(current)?;

        loop {
            // 1. If current has children, move to first child
            if let Some(first_child) = self.first_child_of(curr.id) {
                curr = first_child;
            } else {
                // 2. Otherwise try next sibling; if none, climb parents until an ancestor has a next sibling
                let mut climbed = curr;
                loop {
                    if let Some(next_sib) = self.next_sibling_of(climbed.id) {
                        curr = next_sib;
                        break;
                    }
                    // Climb up
                    if let Some(parent) = self.parent_of(climbed.id) {
                        climbed = parent;
                    } else {
                        // Reached the very end of the tree
                        return None;
                    }
                }
            }

            if filter.matches(curr) {
                return Some(curr);
            }
        }
    }

    /// Moves backward in document/reading order (reverse pre-order DFS) to the previous node matching `filter`.
    pub fn previous_in_order(&self, current: NodeId, filter: &NavFilter) -> Option<&AccessibleNode> {
        let mut curr = self.get(current)?;

        loop {
            // 1. If previous sibling exists, descend to its deepest right-most descendant
            if let Some(prev_sib) = self.previous_sibling_of(curr.id) {
                let mut deepest = prev_sib;
                while let Some(last_child) = self.last_child_of(deepest.id) {
                    deepest = last_child;
                }
                curr = deepest;
            } else if let Some(parent) = self.parent_of(curr.id) {
                // 2. Otherwise move up to parent
                curr = parent;
            } else {
                // Reached start of tree
                return None;
            }

            if filter.matches(curr) {
                return Some(curr);
            }
        }
    }

    // --- Search & Spatial Queries ---

    /// Finds all nodes with the matching role.
    pub fn find_by_role(&self, role: Role) -> Vec<&AccessibleNode> {
        self.nodes.values().filter(|n| n.role == role).collect()
    }

    /// Finds all nodes containing the specified substring in their name or value (case-insensitive).
    pub fn find_by_text(&self, text: &str) -> Vec<&AccessibleNode> {
        let query = text.to_lowercase();
        self.nodes
            .values()
            .filter(|n| {
                n.name.as_deref().map_or(false, |s| s.to_lowercase().contains(&query))
                    || n.value.as_deref().map_or(false, |s| s.to_lowercase().contains(&query))
            })
            .collect()
    }

    /// Finds the leaf-most accessible node whose screen coordinates enclose the given point `(x, y)`.
    pub fn find_at_point(&self, x: f64, y: f64) -> Option<&AccessibleNode> {
        let root = self.root()?;
        if !root.bounds.map_or(false, |b| b.contains_point(x, y)) {
            return None;
        }

        let mut current = root;
        'descend: loop {
            // Check children for a tighter match
            for child in self.children_of(current.id) {
                if child.bounds.map_or(false, |b| b.contains_point(x, y)) {
                    current = child;
                    continue 'descend;
                }
            }
            // No child contains the point; current is the leaf hit
            return Some(current);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{AccessibleAction, ActionCapabilities};
    use crate::node::{NodeId, Rect};
    use crate::roles::Role;

    fn make_node(id: u64, role: Role, name: &str) -> AccessibleNode {
        let mut node = AccessibleNode::new(NodeId(id), role);
        node.name = Some(name.to_string());
        node
    }

    #[test]
    fn test_tree_construction_and_relations() {
        let mut tree = AccessibilityTree::new();

        // Build hierarchy:
        // Window (1)
        //  ├── Dialog (2)
        //  │    ├── Button "OK" (3)
        //  │    └── Button "Cancel" (4)
        //  └── Heading "Welcome" (5)

        let root = make_node(1, Role::Application, "MainApp");
        tree.insert(root);

        let dialog = make_node(2, Role::Dialog, "SettingsDialog");
        tree.attach_child(NodeId(1), dialog);

        let ok_btn = make_node(3, Role::Button, "OK");
        tree.attach_child(NodeId(2), ok_btn);

        let cancel_btn = make_node(4, Role::Button, "Cancel");
        tree.attach_child(NodeId(2), cancel_btn);

        let heading = make_node(5, Role::Heading, "Welcome");
        tree.attach_child(NodeId(1), heading);

        // Verify root
        assert_eq!(tree.root().unwrap().name.as_deref(), Some("MainApp"));
        assert_eq!(tree.len(), 5);

        // Verify parent relations
        assert_eq!(tree.parent_of(NodeId(3)).unwrap().id, NodeId(2));
        assert_eq!(tree.parent_of(NodeId(2)).unwrap().id, NodeId(1));
        assert!(tree.parent_of(NodeId(1)).is_none());

        // Verify sibling relations
        assert_eq!(tree.next_sibling_of(NodeId(3)).unwrap().id, NodeId(4));
        assert!(tree.next_sibling_of(NodeId(4)).is_none());
        assert_eq!(tree.previous_sibling_of(NodeId(4)).unwrap().id, NodeId(3));
        assert!(tree.previous_sibling_of(NodeId(3)).is_none());

        // Verify first and last child
        assert_eq!(tree.first_child_of(NodeId(2)).unwrap().id, NodeId(3));
        assert_eq!(tree.last_child_of(NodeId(2)).unwrap().id, NodeId(4));

        // Verify ancestors
        let ancestors = tree.ancestors_of(NodeId(3));
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0].id, NodeId(2));
        assert_eq!(ancestors[1].id, NodeId(1));

        // Verify descendants of root
        let descendants = tree.descendants_of(NodeId(1));
        assert_eq!(descendants.len(), 4);
        assert_eq!(descendants[0].id, NodeId(2));
        assert_eq!(descendants[1].id, NodeId(3));
        assert_eq!(descendants[2].id, NodeId(4));
        assert_eq!(descendants[3].id, NodeId(5));
    }

    #[test]
    fn test_linear_reading_order_dfs() {
        let mut tree = AccessibilityTree::new();
        tree.insert(make_node(1, Role::Application, "Root"));
        tree.attach_child(NodeId(1), make_node(2, Role::Pane, "Pane"));
        tree.attach_child(NodeId(2), make_node(3, Role::Button, "B1"));
        tree.attach_child(NodeId(2), make_node(4, Role::Button, "B2"));
        tree.attach_child(NodeId(1), make_node(5, Role::StaticText, "Text"));

        // Forward reading order from Root:
        let n1 = tree.next_in_order(NodeId(1), &NavFilter::All).unwrap();
        assert_eq!(n1.id, NodeId(2));
        let n2 = tree.next_in_order(NodeId(2), &NavFilter::All).unwrap();
        assert_eq!(n2.id, NodeId(3));
        let n3 = tree.next_in_order(NodeId(3), &NavFilter::All).unwrap();
        assert_eq!(n3.id, NodeId(4));
        let n4 = tree.next_in_order(NodeId(4), &NavFilter::All).unwrap();
        assert_eq!(n4.id, NodeId(5));
        assert!(tree.next_in_order(NodeId(5), &NavFilter::All).is_none());

        // Reverse reading order from Text (5):
        let p1 = tree.previous_in_order(NodeId(5), &NavFilter::All).unwrap();
        assert_eq!(p1.id, NodeId(4));
        let p2 = tree.previous_in_order(NodeId(4), &NavFilter::All).unwrap();
        assert_eq!(p2.id, NodeId(3));
        let p3 = tree.previous_in_order(NodeId(3), &NavFilter::All).unwrap();
        assert_eq!(p3.id, NodeId(2));
        let p4 = tree.previous_in_order(NodeId(2), &NavFilter::All).unwrap();
        assert_eq!(p4.id, NodeId(1));
        assert!(tree.previous_in_order(NodeId(1), &NavFilter::All).is_none());
    }

    #[test]
    fn test_navigation_filter_by_role() {
        let mut tree = AccessibilityTree::new();
        tree.insert(make_node(1, Role::Application, "Root"));
        tree.attach_child(NodeId(1), make_node(2, Role::Pane, "Pane"));
        tree.attach_child(NodeId(2), make_node(3, Role::Button, "Submit"));
        tree.attach_child(NodeId(2), make_node(4, Role::StaticText, "Label"));
        tree.attach_child(NodeId(1), make_node(5, Role::Heading, "Section Title"));
        tree.attach_child(NodeId(1), make_node(6, Role::Button, "Next"));

        // Filter: only Role::Button
        let button_filter = NavFilter::Role(Role::Button);
        let first_btn = tree.next_in_order(NodeId(1), &button_filter).unwrap();
        assert_eq!(first_btn.id, NodeId(3));
        let second_btn = tree.next_in_order(NodeId(3), &button_filter).unwrap();
        assert_eq!(second_btn.id, NodeId(6));
        assert!(tree.next_in_order(NodeId(6), &button_filter).is_none());

        // Filter: only Heading
        let heading = tree.next_in_order(NodeId(1), &NavFilter::Heading).unwrap();
        assert_eq!(heading.id, NodeId(5));
        assert_eq!(heading.name.as_deref(), Some("Section Title"));
    }

    #[test]
    fn test_spatial_hit_testing() {
        let mut tree = AccessibilityTree::new();

        let mut root = make_node(1, Role::Application, "Window");
        root.bounds = Some(Rect { left: 0.0, top: 0.0, width: 800.0, height: 600.0 });
        tree.insert(root);

        let mut btn = make_node(2, Role::Button, "ClickMe");
        btn.bounds = Some(Rect { left: 50.0, top: 50.0, width: 100.0, height: 40.0 });
        tree.attach_child(NodeId(1), btn);

        // Point inside button
        let hit = tree.find_at_point(60.0, 70.0).unwrap();
        assert_eq!(hit.id, NodeId(2));

        // Point outside button but inside window
        let hit_window = tree.find_at_point(300.0, 300.0).unwrap();
        assert_eq!(hit_window.id, NodeId(1));

        // Point outside window completely
        assert!(tree.find_at_point(900.0, 900.0).is_none());
    }

    #[test]
    fn test_remove_subtree() {
        let mut tree = AccessibilityTree::new();
        tree.insert(make_node(1, Role::Application, "Root"));
        tree.attach_child(NodeId(1), make_node(2, Role::Pane, "Pane"));
        tree.attach_child(NodeId(2), make_node(3, Role::Button, "B1"));
        tree.attach_child(NodeId(2), make_node(4, Role::Button, "B2"));
        tree.attach_child(NodeId(1), make_node(5, Role::Button, "B3"));

        assert_eq!(tree.len(), 5);

        // Remove Pane (2) and all its children (3, 4)
        let removed = tree.remove_subtree(NodeId(2));
        assert_eq!(removed.len(), 3);
        assert_eq!(tree.len(), 2);

        // Children of root should now only contain B3 (5)
        let root = tree.root().unwrap();
        assert_eq!(root.children, vec![NodeId(5)]);
    }

    #[test]
    fn test_android_style_node_actions() {
        let mut node = make_node(10, Role::Button, "OK");
        node.add_action(AccessibleAction::Click);
        node.add_action(AccessibleAction::LongClick);

        assert!(node.action_capabilities.contains(ActionCapabilities::CLICK));
        assert!(node.action_capabilities.contains(ActionCapabilities::LONG_CLICK));
        assert!(!node.action_capabilities.contains(ActionCapabilities::EXPAND));
        assert!(node.is_clickable());
    }
}
