//! Accessible Node and Spatial Geometry Structures.

use crate::roles::Role;
use crate::states::State;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NodeId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PositionInfo {
    pub position_in_set: Option<i32>,
    pub size_of_set: Option<i32>,
    pub row_index: Option<i32>,
    pub column_index: Option<i32>,
    pub level: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AccessibleNode {
    pub id: NodeId,
    pub role: Role,
    pub states: State,
    pub name: Option<String>,
    pub value: Option<String>,
    pub description: Option<String>,
    pub keyboard_shortcut: Option<String>,
    pub class_name: Option<String>,
    pub automation_id: Option<String>,
    pub bounds: Option<Rect>,
    pub position_info: PositionInfo,
    pub process_id: Option<u32>,
}

impl AccessibleNode {
    pub fn new(id: NodeId, role: Role) -> Self {
        Self {
            id,
            role,
            ..Default::default()
        }
    }
}
