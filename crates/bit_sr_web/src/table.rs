//! Web Table and Data Grid traversal subsystem.
//! Supports 2D cell navigation, row/col header association, and layout table detection.

use bit_sr_core::node::NodeId;
use bit_sr_core::roles::Role;
use bit_sr_core::tree::AccessibilityTree;

/// Cell data within a web table.
#[derive(Debug, Clone, PartialEq)]
pub struct TableCellData {
    pub node_id: NodeId,
    pub row: usize,
    pub col: usize,
    pub row_span: usize,
    pub col_span: usize,
    pub text: String,
    pub is_header: bool,
}

/// A parsed 2D web table matrix.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WebTable {
    pub table_node_id: NodeId,
    pub row_count: usize,
    pub col_count: usize,
    pub cells: Vec<TableCellData>,
    pub is_layout_table: bool,
}

impl WebTable {
    /// Returns the cell at (row, col) coordinates, if present.
    pub fn cell_at(&self, row: usize, col: usize) -> Option<&TableCellData> {
        self.cells
            .iter()
            .find(|c| c.row <= row && row < c.row + c.row_span && c.col <= col && col < c.col + c.col_span)
    }

    /// Finds the column header for a given column index.
    pub fn column_header(&self, col: usize) -> Option<String> {
        self.cells
            .iter()
            .find(|c| c.col == col && c.is_header)
            .map(|c| c.text.clone())
    }

    /// Finds the row header for a given row index.
    pub fn row_header(&self, row: usize) -> Option<String> {
        self.cells
            .iter()
            .find(|c| c.row == row && c.is_header)
            .map(|c| c.text.clone())
    }
}

/// Table navigation coordinator for moving across rows and columns.
#[derive(Debug, Clone, Default)]
pub struct TableTraverser {
    pub table: WebTable,
    pub current_row: usize,
    pub current_col: usize,
}

impl TableTraverser {
    /// Creates a traverser for a given table.
    pub fn new(table: WebTable) -> Self {
        Self {
            table,
            current_row: 0,
            current_col: 0,
        }
    }

    /// Moves cursor right to the next column.
    pub fn next_column(&mut self) -> Option<String> {
        if self.current_col + 1 < self.table.col_count {
            self.current_col += 1;
            Some(self.describe_current_cell())
        } else {
            None // Edge of table
        }
    }

    /// Moves cursor left to the previous column.
    pub fn prev_column(&mut self) -> Option<String> {
        if self.current_col > 0 {
            self.current_col -= 1;
            Some(self.describe_current_cell())
        } else {
            None
        }
    }

    /// Moves cursor down to the next row.
    pub fn next_row(&mut self) -> Option<String> {
        if self.current_row + 1 < self.table.row_count {
            self.current_row += 1;
            Some(self.describe_current_cell())
        } else {
            None
        }
    }

    /// Moves cursor up to the previous row.
    pub fn prev_row(&mut self) -> Option<String> {
        if self.current_row > 0 {
            self.current_row -= 1;
            Some(self.describe_current_cell())
        } else {
            None
        }
    }

    /// Formats a full spoken description of the current table cell.
    pub fn describe_current_cell(&self) -> String {
        let cell = self.table.cell_at(self.current_row, self.current_col);
        let header = self.table.column_header(self.current_col);

        let cell_text = cell.map(|c| c.text.trim()).unwrap_or("empty");
        let mut desc = format!("Row {}, Column {}", self.current_row + 1, self.current_col + 1);

        if let Some(h) = header {
            if !h.trim().is_empty() && h.trim() != cell_text {
                desc.push_str(&format!(", {}: {}", h.trim(), cell_text));
                return desc;
            }
        }

        desc.push_str(&format!(": {}", cell_text));
        desc
    }

    /// Parses an AccessibleNode table subtree into a WebTable matrix.
    pub fn parse_table(tree: &AccessibilityTree, table_node_id: NodeId) -> Option<WebTable> {
        let table_node = tree.get(table_node_id)?;
        if table_node.role != Role::Table && table_node.role != Role::DataGrid {
            return None;
        }

        let mut cells = Vec::new();
        let mut row_idx = 0;
        let mut max_col = 0;

        for &child_id in &table_node.children {
            if let Some(row_node) = tree.get(child_id) {
                if row_node.role == Role::TableRow {
                    let mut col_idx = 0;
                    for &cell_id in &row_node.children {
                        if let Some(cell_node) = tree.get(cell_id) {
                            if matches!(
                                cell_node.role,
                                Role::TableCell | Role::TableColumnHeader | Role::TableRowHeader
                            ) {
                                let is_header = cell_node.role == Role::TableColumnHeader
                                    || cell_node.role == Role::TableRowHeader
                                    || cell_node
                                        .collection_item_info
                                        .map(|info| info.is_heading)
                                        .unwrap_or(false);

                                let text = cell_node
                                    .name
                                    .clone()
                                    .or_else(|| cell_node.value.clone())
                                    .unwrap_or_default();

                                cells.push(TableCellData {
                                    node_id: cell_id,
                                    row: row_idx,
                                    col: col_idx,
                                    row_span: 1,
                                    col_span: 1,
                                    text,
                                    is_header,
                                });

                                col_idx += 1;
                                if col_idx > max_col {
                                    max_col = col_idx;
                                }
                            }
                        }
                    }
                    row_idx += 1;
                }
            }
        }

        // Layout table heuristic: tables with 1 row or 1 column and 0 headers are layout tables
        let is_layout = (row_idx <= 1 && max_col <= 1) || cells.iter().all(|c| !c.is_header && c.text.trim().is_empty());

        Some(WebTable {
            table_node_id,
            row_count: row_idx,
            col_count: max_col,
            cells,
            is_layout_table: is_layout,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_table_navigation() {
        let table = WebTable {
            table_node_id: NodeId(10),
            row_count: 2,
            col_count: 2,
            cells: vec![
                TableCellData {
                    node_id: NodeId(11),
                    row: 0,
                    col: 0,
                    row_span: 1,
                    col_span: 1,
                    text: "Product".to_string(),
                    is_header: true,
                },
                TableCellData {
                    node_id: NodeId(12),
                    row: 0,
                    col: 1,
                    row_span: 1,
                    col_span: 1,
                    text: "Price".to_string(),
                    is_header: true,
                },
                TableCellData {
                    node_id: NodeId(13),
                    row: 1,
                    col: 0,
                    row_span: 1,
                    col_span: 1,
                    text: "Apple".to_string(),
                    is_header: false,
                },
                TableCellData {
                    node_id: NodeId(14),
                    row: 1,
                    col: 1,
                    row_span: 1,
                    col_span: 1,
                    text: "$1.50".to_string(),
                    is_header: false,
                },
            ],
            is_layout_table: false,
        };

        let mut traverser = TableTraverser::new(table);
        traverser.current_row = 1;
        traverser.current_col = 0;

        assert_eq!(
            traverser.describe_current_cell(),
            "Row 2, Column 1, Product: Apple"
        );

        let desc_col = traverser.next_column().unwrap();
        assert_eq!(desc_col, "Row 2, Column 2, Price: $1.50");
        assert!(traverser.next_column().is_none()); // End of row
    }
}
