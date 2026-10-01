//! UI Automation Tree Walker Navigation.
//! Aligned with WINDOWS.md Section 3.4.

use windows::Win32::UI::Accessibility::{IUIAutomationCacheRequest, IUIAutomationElement, IUIAutomationTreeWalker};

pub struct TreeNavigator {
    walker: IUIAutomationTreeWalker,
    cache_request: IUIAutomationCacheRequest,
}

impl TreeNavigator {
    pub fn new(walker: IUIAutomationTreeWalker, cache_request: IUIAutomationCacheRequest) -> Self {
        Self { walker, cache_request }
    }

    /// Navigates up to the parent element with cached properties.
    pub fn get_parent(&self, element: &IUIAutomationElement) -> Option<IUIAutomationElement> {
        unsafe {
            self.walker
                .GetParentElementBuildCache(element, &self.cache_request)
                .ok()
        }
    }

    /// Navigates to the first child element with cached properties.
    pub fn get_first_child(&self, element: &IUIAutomationElement) -> Option<IUIAutomationElement> {
        unsafe {
            self.walker
                .GetFirstChildElementBuildCache(element, &self.cache_request)
                .ok()
        }
    }

    /// Navigates to the next sibling element with cached properties.
    pub fn get_next_sibling(&self, element: &IUIAutomationElement) -> Option<IUIAutomationElement> {
        unsafe {
            self.walker
                .GetNextSiblingElementBuildCache(element, &self.cache_request)
                .ok()
        }
    }

    /// Navigates to the previous sibling element with cached properties.
    pub fn get_previous_sibling(&self, element: &IUIAutomationElement) -> Option<IUIAutomationElement> {
        unsafe {
            self.walker
                .GetPreviousSiblingElementBuildCache(element, &self.cache_request)
                .ok()
        }
    }

    /// Recursively harvests an in-memory AccessibilityTree scoped to `root_element`.
    /// Strictly limits traversal depth and maximum node count to prevent freezing (Invariant 3).
    pub fn harvest_subtree(
        &self,
        root_element: &IUIAutomationElement,
        max_depth: usize,
        max_nodes: usize,
    ) -> bit_sr_core::tree::AccessibilityTree {
        let mut tree = bit_sr_core::tree::AccessibilityTree::new();
        let root_node = crate::uia::UiaElement::new(root_element.clone()).to_accessible_node();
        let root_id = root_node.id;
        tree.insert(root_node);

        self.harvest_children(root_element, root_id, 1, max_depth, max_nodes, &mut tree);
        tree
    }

    fn harvest_children(
        &self,
        parent_element: &IUIAutomationElement,
        parent_id: bit_sr_core::node::NodeId,
        current_depth: usize,
        max_depth: usize,
        max_nodes: usize,
        tree: &mut bit_sr_core::tree::AccessibilityTree,
    ) {
        if current_depth > max_depth || tree.len() >= max_nodes {
            return;
        }

        let mut child = self.get_first_child(parent_element);
        while let Some(current_child) = child {
            if tree.len() >= max_nodes {
                break;
            }

            let node = crate::uia::UiaElement::new(current_child.clone()).to_accessible_node();
            let child_id = node.id;

            tree.attach_child(parent_id, node);

            // Recurse into children
            self.harvest_children(
                &current_child,
                child_id,
                current_depth + 1,
                max_depth,
                max_nodes,
                tree,
            );

            child = self.get_next_sibling(&current_child);
        }
    }
}
