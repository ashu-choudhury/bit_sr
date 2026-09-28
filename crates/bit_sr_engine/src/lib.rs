//! Core Screen Reader Engine for bit_sr.
//!
//! Platform-agnostic engine orchestrating speech formatting, focus tracking,
//! keyboard command dispatching, and reactive event loops.

pub mod commands;
pub mod coordinator;
pub mod formatter;
pub mod tracker;

pub use commands::{CommandDispatcher, ScreenReaderCommand, SpeechMode};
pub use coordinator::{EngineAction, EngineCoordinator};
pub use formatter::{FormatterContext, SpeechFormatter};
pub use tracker::{FocusTracker, FocusTransition};
