//! JNI Array and String Conversion Utilities.
//! Zero-allocation primitive buffers and UTF-8 string decoding helpers.

use jni::objects::{JObjectArray, JString};
use jni::JNIEnv;

/// Safely extracts a Rust `String` from a `JString` reference.
pub fn jstring_to_string(env: &mut JNIEnv, jstr: &JString) -> String {
    if jstr.is_null() {
        String::new()
    } else {
        env.get_string(jstr).map(|s| s.into()).unwrap_or_default()
    }
}

/// Safely extracts a Rust `String` from an element in a `JObjectArray`.
pub fn jobject_array_string(env: &mut JNIEnv, array: &JObjectArray, index: usize) -> String {
    match env.get_object_array_element(array, index as i32) {
        Ok(obj) if !obj.is_null() => {
            let jstr = JString::from(obj);
            env.get_string(&jstr).map(|s| s.into()).unwrap_or_default()
        }
        _ => String::new(),
    }
}
