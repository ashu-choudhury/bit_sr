//! Linearizer: Compiles an in-memory AccessibilityTree into a navigable VirtualBuffer.
//! Handles HTML/ARIA block vs inline boundaries, heading extraction, and interactive controls.

use crate::buffer::{BufferLine, TextRun, VirtualBuffer};
use bit_sr_core::node::{AccessibleNode, NodeId};
use bit_sr_core::roles::Role;
use bit_sr_core::tree::AccessibilityTree;

/// Compiles hierarchical DOM and accessibility subtrees into a flattened VirtualBuffer.
pub struct Linearizer;

impl Linearizer {
    /// Compiles an entire accessibility tree anchored at `root_node_id` into a VirtualBuffer.
    pub fn compile(tree: &AccessibilityTree, root_node_id: NodeId) -> VirtualBuffer {
        let mut lines = Vec::new();
        let mut current_runs = Vec::new();

        if let Some(root) = tree.get(root_node_id) {
            Self::traverse_node(tree, root, &mut lines, &mut current_runs);
        }

        // Flush any trailing runs
        if !current_runs.is_empty() {
            lines.push(BufferLine::new(lines.len(), current_runs));
        }

        // Clean up empty lines and normalize line numbers
        let clean_lines: Vec<BufferLine> = lines
            .into_iter()
            .filter(|line| !line.is_empty())
            .enumerate()
            .map(|(idx, mut line)| {
                line.line_number = idx;
                line
            })
            .collect();

        let mut buffer = VirtualBuffer::with_lines(root_node_id, clean_lines);
        if let Some(root) = tree.get(root_node_id) {
            buffer.title = root.name.clone();
        }
        buffer
    }

    /// Recursively traverses a node, distinguishing block boundaries from inline content.
    fn traverse_node(
        tree: &AccessibilityTree,
        node: &AccessibleNode,
        lines: &mut Vec<BufferLine>,
        current_runs: &mut Vec<TextRun>,
    ) {
        let is_block = Self::is_block_element(node);

        // Before starting a block element, break to a new line if current line has content
        if is_block && !current_runs.is_empty() {
            let line_idx = lines.len();
            lines.push(BufferLine::new(line_idx, std::mem::take(current_runs)));
        }

        // If this node itself produces readable text run(s)
        if let Some(run) = Self::create_run_from_node(node, current_runs) {
            current_runs.push(run);
        }

        // Traverse children in DOM reading order
        for &child_id in &node.children {
            if let Some(child) = tree.get(child_id) {
                Self::traverse_node(tree, child, lines, current_runs);
            }
        }

        // After completing a block element, flush line to ensure separation
        if is_block && !current_runs.is_empty() {
            let line_idx = lines.len();
            lines.push(BufferLine::new(line_idx, std::mem::take(current_runs)));
        }
    }

    /// Determines if an accessible role acts as a block-level container in reading order.
    pub fn is_block_element(node: &AccessibleNode) -> bool {
        matches!(
            node.role,
            Role::Heading
                | Role::Paragraph
                | Role::Table
                | Role::TableRow
                | Role::List
                | Role::ListItem
                | Role::Landmark
                | Role::Section
                | Role::Form
                | Role::Document
                | Role::Box
                | Role::Dialog
                | Role::Alert
                | Role::Separator
                | Role::Header
                | Role::StatusBar
                | Role::MenuBar
                | Role::ToolBar
        )
    }

    /// Creates a TextRun from a node if it represents printable or interactive content.
    fn create_run_from_node(node: &AccessibleNode, current_runs: &[TextRun]) -> Option<TextRun> {
        let text_opt = node.name.as_deref().or(node.value.as_deref());

        // Extract heading level
        let heading_level = if node.role == Role::Heading || node.is_heading() {
            node.position_info
                .level
                .map(|l| (l.clamp(1, 6)) as u8)
                .or(Some(2)) // Default heading level 2 if unspecified
        } else {
            None
        };

        match node.role {
            Role::Heading => {
                let text = text_opt.unwrap_or("").trim();
                if !text.is_empty() {
                    let level = heading_level.unwrap_or(2);
                    Some(TextRun::new_heading(node.id, text, level))
                } else {
                    None
                }
            }

            Role::Link => {
                let text = text_opt.unwrap_or("").trim();
                if !text.is_empty() {
                    Some(TextRun {
                        text: text.to_string(),
                        node_id: node.id,
                        role: Role::Link,
                        states: node.states,
                        tag: Some("a".to_string()),
                        href: node.value.clone(),
                        heading_level: None,
                        is_clickable: true,
                        bounds: node.bounds,
                    })
                } else {
                    None
                }
            }

            Role::Button | Role::SplitButton | Role::DropDownButton => {
                let text = text_opt.unwrap_or("").trim();
                if !text.is_empty() {
                    Some(TextRun {
                        text: text.to_string(),
                        node_id: node.id,
                        role: node.role,
                        states: node.states,
                        tag: Some("button".to_string()),
                        href: None,
                        heading_level: None,
                        is_clickable: true,
                        bounds: node.bounds,
                    })
                } else {
                    None
                }
            }

            Role::CheckBox | Role::RadioButton | Role::Switch => {
                let text = text_opt.unwrap_or("").trim();
                if !text.is_empty() {
                    Some(TextRun {
                        text: text.to_string(),
                        node_id: node.id,
                        role: node.role,
                        states: node.states,
                        tag: Some("input".to_string()),
                        href: None,
                        heading_level: None,
                        is_clickable: true,
                        bounds: node.bounds,
                    })
                } else {
                    None
                }
            }

            Role::EditableText => {
                let label = text_opt.unwrap_or("").trim();
                let val = node.value.as_deref();
                Some(TextRun::new_edit(node.id, label, val))
            }

            Role::ComboBox => {
                let label = text_opt.unwrap_or("").trim();
                let val_suffix = node
                    .value
                    .as_deref()
                    .map(|v| format!(": {}", v))
                    .unwrap_or_default();
                Some(TextRun {
                    text: format!("{}{}", label, val_suffix),
                    node_id: node.id,
                    role: Role::ComboBox,
                    states: node.states,
                    tag: Some("select".to_string()),
                    href: None,
                    heading_level: None,
                    is_clickable: true,
                    bounds: node.bounds,
                })
            }

            Role::Graphic => {
                let alt_text = text_opt.unwrap_or("").trim();
                if !alt_text.is_empty() {
                    Some(TextRun {
                        text: alt_text.to_string(),
                        node_id: node.id,
                        role: Role::Graphic,
                        states: node.states,
                        tag: Some("img".to_string()),
                        href: None,
                        heading_level: None,
                        is_clickable: false,
                        bounds: node.bounds,
                    })
                } else {
                    None
                }
            }

            Role::StaticText => {
                let text = text_opt.unwrap_or("").trim();
                if !text.is_empty() {
                    // Anti-duplication: avoid re-adding identical text if the immediate previous run
                    // was a Heading, Button, or Link that already incorporated this text.
                    if let Some(last_run) = current_runs.last() {
                        if last_run.text.trim() == text
                            && matches!(last_run.role, Role::Heading | Role::Button | Role::Link)
                        {
                            return None;
                        }
                    }
                    Some(TextRun {
                        text: text.to_string(),
                        node_id: node.id,
                        role: Role::StaticText,
                        states: node.states,
                        tag: None,
                        href: None,
                        heading_level: None,
                        is_clickable: false,
                        bounds: node.bounds,
                    })
                } else {
                    None
                }
            }

            Role::Separator => Some(TextRun {
                text: "---".to_string(),
                node_id: node.id,
                role: Role::Separator,
                states: node.states,
                tag: Some("hr".to_string()),
                href: None,
                heading_level: None,
                is_clickable: false,
                bounds: node.bounds,
            }),

            _ => {
                // If it's a generic node with explicit text and no children, treat as text
                if node.children.is_empty() {
                    if let Some(text) = text_opt {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            return Some(TextRun {
                                text: trimmed.to_string(),
                                node_id: node.id,
                                role: node.role,
                                states: node.states,
                                tag: None,
                                href: None,
                                heading_level,
                                is_clickable: false,
                                bounds: node.bounds,
                            });
                        }
                    }
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit_sr_core::node::PositionInfo;

    #[test]
    fn test_linearizer_compilation() {
        let mut tree = AccessibilityTree::new();

        let doc_id = NodeId(1);
        let mut doc_node = AccessibleNode::default();
        doc_node.id = doc_id;
        doc_node.role = Role::Document;
        doc_node.name = Some("Sample Page".to_string());
        tree.insert(doc_node);

        let h1_id = NodeId(2);
        let mut h1_node = AccessibleNode::default();
        h1_node.id = h1_id;
        h1_node.role = Role::Heading;
        h1_node.name = Some("Welcome to Bit SR".to_string());
        h1_node.position_info = PositionInfo {
            level: Some(1),
            ..Default::default()
        };
        tree.attach_child(doc_id, h1_node);

        let p_id = NodeId(3);
        let mut p_node = AccessibleNode::default();
        p_id_setup(&mut p_node, p_id);
        tree.attach_child(doc_id, p_node);

        let link_id = NodeId(4);
        let mut link_node = AccessibleNode::default();
        link_node.id = link_id;
        link_node.role = Role::Link;
        link_node.name = Some("Read documentation".to_string());
        link_node.value = Some("https://example.com/docs".to_string());
        tree.attach_child(p_id, link_node);

        let btn_id = NodeId(5);
        let mut btn_node = AccessibleNode::default();
        btn_node.id = btn_id;
        btn_node.role = Role::Button;
        btn_node.name = Some("Get Started".to_string());
        tree.attach_child(doc_id, btn_node);

        let buffer = Linearizer::compile(&tree, doc_id);

        assert_eq!(buffer.title.as_deref(), Some("Sample Page"));
        assert!(buffer.line_count() >= 3);

        // First line is Heading 1
        let line0 = &buffer.lines[0];
        assert_eq!(line0.spoken_text(), "heading level 1 Welcome to Bit SR");

        // Link line
        let line1 = &buffer.lines[1];
        assert_eq!(line1.spoken_text(), "link Read documentation");

        // Button line
        let line2 = &buffer.lines[2];
        assert_eq!(line2.spoken_text(), "Get Started, button");
    }

    fn p_id_setup(node: &mut AccessibleNode, id: NodeId) {
        node.id = id;
        node.role = Role::Paragraph;
    }

    #[test]
    fn test_linearizer_deduplicates_child_static_text() {
        let mut tree = AccessibilityTree::new();
        let doc_id = NodeId(1);
        let mut doc = AccessibleNode::default();
        doc.id = doc_id;
        doc.role = Role::Document;
        tree.insert(doc);

        let h_id = NodeId(2);
        let mut h_node = AccessibleNode::default();
        h_node.id = h_id;
        h_node.role = Role::Heading;
        h_node.name = Some("Installation Guide".to_string());
        h_node.position_info = PositionInfo { level: Some(2), ..Default::default() };
        tree.attach_child(doc_id, h_node);

        // Child StaticText with the exact same text as the heading
        let text_id = NodeId(3);
        let mut text_node = AccessibleNode::default();
        text_node.id = text_id;
        text_node.role = Role::StaticText;
        text_node.name = Some("Installation Guide".to_string());
        tree.attach_child(h_id, text_node);

        let buffer = Linearizer::compile(&tree, doc_id);
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(buffer.lines[0].spoken_text(), "heading level 2 Installation Guide");
    }
}
