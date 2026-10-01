//! `bit_sr_web`: Universal Web, WebView, and Virtual Buffer Subsystem.
//!
//! Provides platform-agnostic models and controllers for:
//! - In-memory linearized `VirtualBuffer` for web documents.
//! - Browse Mode (virtual cursor) vs Focus Mode (forms mode) state machine.
//! - Single-letter quick navigation (`H`, `K`, `F`, `B`, `T`, `L`, etc.).
//! - 2D Table and Data Grid cell traversal.
//! - W3C ARIA 1.2/1.3 semantics, landmarks, and live region tracking.
//! - Universal web text heuristics and DOM tree linearizer.

pub mod aria;
pub mod buffer;
pub mod controller;
pub mod heuristics;
pub mod linearizer;
pub mod quick_nav;
pub mod table;

pub use aria::{AriaLandmark, LiveRegionTracker, LiveRegionUpdate};
pub use buffer::{BufferLine, NavigationMode, TextRun, VirtualBuffer};
pub use controller::{WebAction, WebController};
pub use heuristics::WebHeuristics;
pub use linearizer::Linearizer;
pub use quick_nav::{QuickNav, QuickNavKey};
pub use table::{TableCellData, TableTraverser, WebTable};
