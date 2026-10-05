//! JNI Bridge Subsystem and Conversion Utilities.
//! Provides zero-allocation string and array converters for the unified engine.

pub mod converters;

pub use converters::{jobject_array_string, jstring_to_string};
