//! Ultra-low-latency native Android accessibility platform backend for `bit_sr`.
//! Engineered symmetrically with `bit_sr_platform_windows` for clean-room performance and modularity.

pub mod apps;
pub mod error;
pub mod feedback;
pub mod input;
pub mod jni;
pub mod platform;
pub mod tree;

// Re-exports of primary types
pub use apps::{AndroidAppType, AndroidChromeFilter, SoftKeyboardFilter, SystemUiFilter};
pub use error::{Error, Result};
pub use feedback::{AndroidTtsDriver, EarconType, HapticEffect};
pub use input::{HardwareKeyTracker, KeyAction, SwipeDirection, TouchResult, TouchStateMachine};
pub use platform::AndroidPlatform;
pub use tree::{map_class_name_to_role, unpack_states, CachedNode, SpatialNodeCache};
