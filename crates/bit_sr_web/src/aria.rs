//! W3C ARIA 1.2 / 1.3 semantics and live region handling.

use bit_sr_core::node::{LiveRegion, NodeId};
use std::collections::HashMap;

/// ARIA Landmark types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AriaLandmark {
    Banner,
    Complementary,
    ContentInfo,
    Form,
    Main,
    Navigation,
    Region,
    Search,
}

impl AriaLandmark {
    /// Spoken label for entering a landmark region.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Banner => "banner landmark",
            Self::Complementary => "complementary landmark",
            Self::ContentInfo => "content info landmark",
            Self::Form => "form landmark",
            Self::Main => "main landmark",
            Self::Navigation => "navigation landmark",
            Self::Region => "region landmark",
            Self::Search => "search landmark",
        }
    }

    /// Parses landmark from HTML/ARIA string attribute.
    pub fn from_role_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "banner" => Some(Self::Banner),
            "complementary" => Some(Self::Complementary),
            "contentinfo" => Some(Self::ContentInfo),
            "form" => Some(Self::Form),
            "main" => Some(Self::Main),
            "navigation" => Some(Self::Navigation),
            "region" => Some(Self::Region),
            "search" => Some(Self::Search),
            _ => None,
        }
    }
}

/// An update event emitted from an ARIA live region.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveRegionUpdate {
    pub node_id: NodeId,
    pub live: LiveRegion,
    pub announcement: String,
}

/// In-memory tracker for ARIA live regions across the active document.
#[derive(Debug, Clone, Default)]
pub struct LiveRegionTracker {
    /// Maps live region node IDs to their last observed text content.
    last_seen_texts: HashMap<NodeId, String>,
}

impl LiveRegionTracker {
    pub fn new() -> Self {
        Self {
            last_seen_texts: HashMap::new(),
        }
    }

    /// Evaluates a live region node. If content has changed, returns an announcement update.
    pub fn evaluate_change(
        &mut self,
        node_id: NodeId,
        live: LiveRegion,
        current_text: &str,
    ) -> Option<LiveRegionUpdate> {
        if live == LiveRegion::None {
            return None;
        }

        let trimmed = current_text.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Some(prev) = self.last_seen_texts.get(&node_id) {
            if prev == trimmed {
                return None; // No change
            }
        }

        self.last_seen_texts.insert(node_id, trimmed.to_string());
        Some(LiveRegionUpdate {
            node_id,
            live,
            announcement: trimmed.to_string(),
        })
    }

    /// Clears tracking cache on document reload.
    pub fn clear(&mut self) {
        self.last_seen_texts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_region_tracker() {
        let mut tracker = LiveRegionTracker::new();
        let node = NodeId(99);

        // First event
        let update1 = tracker.evaluate_change(node, LiveRegion::Polite, "Download started");
        assert!(update1.is_some());
        assert_eq!(update1.unwrap().announcement, "Download started");

        // Duplicate text - suppressed
        let update2 = tracker.evaluate_change(node, LiveRegion::Polite, "Download started");
        assert!(update2.is_none());

        // Mutated text - announces
        let update3 = tracker.evaluate_change(node, LiveRegion::Polite, "Download completed");
        assert!(update3.is_some());
        assert_eq!(update3.unwrap().announcement, "Download completed");
    }
}
