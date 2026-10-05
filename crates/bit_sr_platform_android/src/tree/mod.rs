//! Android Accessibility Tree, Node Cache, and Semantic Mapping Subsystem.

pub mod cache;
pub mod mapper;
pub mod node;

pub use cache::SpatialNodeCache;
pub use mapper::{map_class_name_to_role, unpack_states, STATE_ACCESSIBILITY_FOCUSED, STATE_CHECKED, STATE_DISABLED, STATE_FOCUSED, STATE_SELECTED};
pub use node::CachedNode;
