//! Text-to-Speech Engine Drivers.

pub mod mock;
pub use mock::MockSynthesizer;

#[cfg(windows)]
pub mod sapi5;
#[cfg(windows)]
pub use sapi5::Sapi5Synthesizer;
