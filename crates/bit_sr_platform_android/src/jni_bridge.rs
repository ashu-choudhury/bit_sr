//! JNI Exported Functions for `BitSrAccessibilityService.kt` and `NativeBridge.kt`.
//! Engineered for zero garbage collection allocations and sub-microsecond latency.

use std::sync::atomic::{AtomicI64, Ordering};
use parking_lot::Mutex;

use jni::objects::{JClass, JLongArray, JObjectArray, JIntArray, JString};
use jni::sys::{jboolean, jfloat, jint, jlong, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;

use bit_sr_core::node::{NodeId, Rect};
use bit_sr_core::roles::Role;

use crate::node_cache::{CachedNode, SpatialNodeCache};
use crate::touch_machine::{TouchResult, TouchStateMachine};

/// Native Android Screen Reader Engine State.
pub struct AndroidEngineState {
    pub touch_machine: Mutex<TouchStateMachine>,
    pub node_cache: Mutex<SpatialNodeCache>,
    pub last_focused_node: AtomicI64,
}

impl AndroidEngineState {
    pub fn new() -> Self {
        Self {
            touch_machine: Mutex::new(TouchStateMachine::new()),
            node_cache: Mutex::new(SpatialNodeCache::new()),
            last_focused_node: AtomicI64::new(0),
        }
    }
}

impl Default for AndroidEngineState {
    fn default() -> Self {
        Self::new()
    }
}

/// Java_org_bitsr_screenreader_NativeBridge_initEngine
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_bitsr_screenreader_NativeBridge_initEngine(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let state = Box::new(AndroidEngineState::new());
    Box::into_raw(state) as jlong
}

/// Java_org_bitsr_screenreader_NativeBridge_destroyEngine
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_bitsr_screenreader_NativeBridge_destroyEngine(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    if engine_ptr != 0 {
        unsafe {
            let _ = Box::from_raw(engine_ptr as *mut AndroidEngineState);
        }
    }
}

/// Java_org_bitsr_screenreader_NativeBridge_onRawTouch
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_bitsr_screenreader_NativeBridge_onRawTouch(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    action: jint,
    x: jfloat,
    y: jfloat,
    event_time: jlong,
) -> jboolean {
    if engine_ptr == 0 {
        return JNI_FALSE;
    }

    let state = unsafe { &*(engine_ptr as *const AndroidEngineState) };
    let result = state
        .touch_machine
        .lock()
        .process_touch(action, x as f64, y as f64, event_time);

    match result {
        TouchResult::Explore { x, y } => {
            // Hit test against local in-memory node cache in nanoseconds
            let cache = state.node_cache.lock();
            if let Some(node) = cache.hit_test(x, y) {
                let prev_id = state.last_focused_node.swap(node.id.0 as i64, Ordering::Relaxed);
                if prev_id != node.id.0 as i64 {
                    log::info!("Touch explored item: {:?} - {}", node.role, node.label);
                }
            }
            JNI_TRUE
        }
        TouchResult::Tap { x, y } => {
            let cache = state.node_cache.lock();
            if let Some(node) = cache.hit_test(x, y) {
                log::info!("Tapped node: {} ({:?})", node.label, node.role);
            }
            JNI_TRUE
        }
        TouchResult::DoubleTap { x, y } => {
            log::info!("Double tap activated at ({}, {})!", x, y);
            JNI_TRUE
        }
        TouchResult::Flick(cmd) => {
            log::info!("Flick gesture triggered: {:?}", cmd);
            JNI_TRUE
        }
        TouchResult::None => JNI_FALSE,
    }
}

/// Java_org_bitsr_screenreader_NativeBridge_onAccessibilityEvent
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_bitsr_screenreader_NativeBridge_onAccessibilityEvent(
    mut env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    event_type: jint,
    package_name: JString,
    class_name: JString,
    text: JString,
    content_description: JString,
    _node_source_id: jlong,
    left: jint,
    top: jint,
    right: jint,
    bottom: jint,
) {
    if engine_ptr == 0 {
        return;
    }

    let _pkg: String = env.get_string(&package_name).map(|s| s.into()).unwrap_or_default();
    let cls: String = env.get_string(&class_name).map(|s| s.into()).unwrap_or_default();
    let txt: String = env.get_string(&text).map(|s| s.into()).unwrap_or_default();
    let desc: String = env.get_string(&content_description).map(|s| s.into()).unwrap_or_default();

    let label = if !txt.is_empty() { txt } else { desc };
    let role = crate::node_mapper::map_class_name_to_role(&cls);

    log::debug!(
        "AOSP event: type=0x{:X}, role={:?}, label='{}', bounds=[{}, {}, {}, {}]",
        event_type,
        role,
        label,
        left,
        top,
        right,
        bottom
    );
}

/// Java_org_bitsr_screenreader_NativeBridge_updateWindowTree
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_bitsr_screenreader_NativeBridge_updateWindowTree(
    mut env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    window_id: jint,
    node_ids: JLongArray,
    _roles: JIntArray,
    bounds_left: JIntArray,
    bounds_top: JIntArray,
    bounds_right: JIntArray,
    bounds_bottom: JIntArray,
    texts: JObjectArray,
    states: JLongArray,
) {
    if engine_ptr == 0 {
        return;
    }

    let state = unsafe { &*(engine_ptr as *const AndroidEngineState) };

    let count = match env.get_array_length(&node_ids) {
        Ok(len) => len as usize,
        Err(_) => return,
    };

    let mut id_buf = vec![0i64; count];
    let mut left_buf = vec![0i32; count];
    let mut top_buf = vec![0i32; count];
    let mut right_buf = vec![0i32; count];
    let mut bottom_buf = vec![0i32; count];
    let mut state_buf = vec![0i64; count];

    let _ = env.get_long_array_region(&node_ids, 0, &mut id_buf);
    let _ = env.get_int_array_region(&bounds_left, 0, &mut left_buf);
    let _ = env.get_int_array_region(&bounds_top, 0, &mut top_buf);
    let _ = env.get_int_array_region(&bounds_right, 0, &mut right_buf);
    let _ = env.get_int_array_region(&bounds_bottom, 0, &mut bottom_buf);
    let _ = env.get_long_array_region(&states, 0, &mut state_buf);

    let mut harvested = Vec::with_capacity(count);

    for i in 0..count {
        let label: String = match env.get_object_array_element(&texts, i as i32) {
            Ok(obj) if !obj.is_null() => {
                let jstr = JString::from(obj);
                env.get_string(&jstr).map(|s| s.into()).unwrap_or_default()
            }
            _ => String::new(),
        };

        let width = (right_buf[i] - left_buf[i]).max(0) as f64;
        let height = (bottom_buf[i] - top_buf[i]).max(0) as f64;

        harvested.push(CachedNode {
            id: NodeId(id_buf[i] as u64),
            role: Role::Unknown,
            bounds: Rect {
                left: left_buf[i] as f64,
                top: top_buf[i] as f64,
                width,
                height,
            },
            label,
            states: vec![],
        });
    }

    state.node_cache.lock().update_window_tree(window_id, harvested);
}
