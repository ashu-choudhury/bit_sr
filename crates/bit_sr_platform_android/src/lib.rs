//! Ultra-low-latency native Android accessibility platform backend for `bit_sr`.

pub mod jni_bridge;
pub mod node_cache;
pub mod node_mapper;
pub mod speech_bridge;
pub mod touch_machine;

pub use node_cache::{CachedNode, SpatialNodeCache};
pub use touch_machine::{TouchResult, TouchStateMachine};
