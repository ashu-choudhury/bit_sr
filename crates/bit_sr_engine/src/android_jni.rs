//! Android JNI Native Bridge Interface.
//! Connects AOSP AccessibilityService, TouchInteractionController, and TextToSpeech
//! directly to the universal EngineCoordinator.

use std::sync::atomic::AtomicI64;
use std::sync::Arc;
use parking_lot::{Mutex, RwLock};

use jni::objects::{JClass, JIntArray, JLongArray, JObjectArray, JString};
use jni::sys::{jboolean, jfloat, jint, jlong, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;

use bit_sr_core::actions::{AccessibleAction, ActionError, ActionPerformer};
use bit_sr_core::events::AccessibilityEvent;
use bit_sr_core::node::{AccessibleNode, NodeId, Rect};
use bit_sr_speech::SpeechHub;

use bit_sr_platform_android::feedback::{
    clear_speech_callback, set_speech_callback, AndroidTtsDriver,
};
use bit_sr_platform_android::input::{SwipeDirection, TouchResult, TouchStateMachine};
use bit_sr_platform_android::tree::{
    map_class_name_to_role, unpack_states, CachedNode, SpatialNodeCache,
};

use crate::coordinator::EngineCoordinator;

static JAVA_VM: RwLock<Option<jni::JavaVM>> = RwLock::new(None);

/// Dispatches speech output up to `org.bitsr.screenreader.NativeBridge.speakText`.
pub fn call_java_speak_text(text: &str, interrupt: bool) {
    if let Some(ref vm) = *JAVA_VM.read() {
        if let Ok(mut env) = vm.attach_current_thread() {
            if let Ok(j_text) = env.new_string(text) {
                let j_interrupt: jboolean = if interrupt { JNI_TRUE } else { JNI_FALSE };
                let _ = env.call_static_method(
                    "org/bitsr/screenreader/NativeBridge",
                    "speakText",
                    "(Ljava/lang/String;Z)V",
                    &[(&j_text).into(), j_interrupt.into()],
                );
            }
        }
    }
}

/// Dispatches action execution up to `org.bitsr.screenreader.NativeBridge.performAction`.
pub fn call_java_perform_action(action_id: i32, node_id: i64) -> bool {
    if let Some(ref vm) = *JAVA_VM.read() {
        if let Ok(mut env) = vm.attach_current_thread() {
            if let Ok(result) = env.call_static_method(
                "org/bitsr/screenreader/NativeBridge",
                "performAction",
                "(IJ)Z",
                &[action_id.into(), node_id.into()],
            ) {
                return result.z().unwrap_or(false);
            }
        }
    }
    false
}

/// Action performer bridge executing actions on Android AccessibilityNodeInfo.
struct AndroidActionPerformer;

impl ActionPerformer for AndroidActionPerformer {
    fn perform_action(&self, target: NodeId, action: &AccessibleAction) -> Result<(), ActionError> {
        let action_id = match action {
            AccessibleAction::Click => 16,            // AccessibilityNodeInfo.ACTION_CLICK (0x10)
            AccessibleAction::LongClick => 32,        // ACTION_LONG_CLICK (0x20)
            AccessibleAction::Focus => 1,             // ACTION_FOCUS (0x1)
            AccessibleAction::ClearFocus => 2,        // ACTION_CLEAR_FOCUS (0x2)
            AccessibleAction::Select => 4,            // ACTION_SELECT (0x4)
            AccessibleAction::ClearSelection => 8,    // ACTION_CLEAR_SELECTION (0x8)
            AccessibleAction::ScrollForward => 4096,  // ACTION_SCROLL_FORWARD (0x1000)
            AccessibleAction::ScrollBackward => 8192, // ACTION_SCROLL_BACKWARD (0x2000)
            _ => 16,
        };
        let success = call_java_perform_action(action_id, target.0 as i64);
        if success {
            Ok(())
        } else {
            Err(ActionError::PlatformFailure(
                "Android native action execution returned false".to_string(),
            ))
        }
    }
}

/// Native Android Screen Reader Engine State.
pub struct AndroidEngineState {
    pub coordinator: Mutex<EngineCoordinator>,
    pub node_cache: Mutex<SpatialNodeCache>,
    pub touch_machine: Mutex<TouchStateMachine>,
    pub last_focused_id: AtomicI64,
}

/// Java_org_bitsr_screenreader_NativeBridge_initEngine
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_bitsr_screenreader_NativeBridge_initEngine(
    env: JNIEnv,
    _class: JClass,
) -> jlong {
    if let Ok(vm) = env.get_java_vm() {
        *JAVA_VM.write() = Some(vm);
    }

    set_speech_callback(|text, interrupt| {
        call_java_speak_text(text, interrupt);
    });

    let mut speech_hub = SpeechHub::new();
    speech_hub.register_driver(Box::new(AndroidTtsDriver::new()));
    let _ = speech_hub.set_active_synthesizer("android_tts");

    let mut coordinator = EngineCoordinator::new(speech_hub);
    coordinator.set_action_performer(Arc::new(AndroidActionPerformer));

    let state = Box::new(AndroidEngineState {
        coordinator: Mutex::new(coordinator),
        node_cache: Mutex::new(SpatialNodeCache::new()),
        touch_machine: Mutex::new(TouchStateMachine::new()),
        last_focused_id: AtomicI64::new(0),
    });

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
        clear_speech_callback();
        unsafe {
            let _ = Box::from_raw(engine_ptr as *mut AndroidEngineState);
        }
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
    node_source_id: jlong,
    left: jint,
    top: jint,
    right: jint,
    bottom: jint,
) {
    if engine_ptr == 0 {
        return;
    }

    let state = unsafe { &*(engine_ptr as *const AndroidEngineState) };

    let _pkg = bit_sr_platform_android::jni::converters::jstring_to_string(&mut env, &package_name);
    let cls = bit_sr_platform_android::jni::converters::jstring_to_string(&mut env, &class_name);
    let txt = bit_sr_platform_android::jni::converters::jstring_to_string(&mut env, &text);
    let desc = bit_sr_platform_android::jni::converters::jstring_to_string(&mut env, &content_description);

    let label = if !txt.is_empty() { txt } else { desc };
    let role = map_class_name_to_role(&cls);

    let width = (right - left).max(0) as f64;
    let height = (bottom - top).max(0) as f64;

    let node = AccessibleNode {
        id: NodeId(node_source_id as u64),
        role,
        name: if label.is_empty() { None } else { Some(label) },
        bounds: Some(Rect {
            left: left as f64,
            top: top as f64,
            width,
            height,
        }),
        ..Default::default()
    };

    const TYPE_VIEW_SELECTED: jint = 0x00000004;
    const TYPE_VIEW_FOCUSED: jint = 0x00000008;
    const TYPE_WINDOW_STATE_CHANGED: jint = 0x00000020;
    const TYPE_VIEW_ACCESSIBILITY_FOCUSED: jint = 0x00008000;

    let mut coord = state.coordinator.lock();
    if event_type == TYPE_VIEW_FOCUSED || event_type == TYPE_VIEW_ACCESSIBILITY_FOCUSED || event_type == TYPE_VIEW_SELECTED {
        state.last_focused_id.store(node_source_id, std::sync::atomic::Ordering::Relaxed);
        coord.handle_event(AccessibilityEvent::Focus(node));
    } else if event_type == TYPE_WINDOW_STATE_CHANGED {
        coord.handle_event(AccessibilityEvent::WindowActivated(node));
    }
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
        let label = bit_sr_platform_android::jni::converters::jobject_array_string(&mut env, &texts, i);
        let width = (right_buf[i] - left_buf[i]).max(0) as f64;
        let height = (bottom_buf[i] - top_buf[i]).max(0) as f64;
        let node_states = unpack_states(state_buf[i]);

        harvested.push(CachedNode {
            id: NodeId(id_buf[i] as u64),
            role: bit_sr_core::roles::Role::Unknown,
            bounds: Rect {
                left: left_buf[i] as f64,
                top: top_buf[i] as f64,
                width,
                height,
            },
            label,
            states: node_states,
        });
    }

    state.node_cache.lock().update_window_tree(window_id, harvested);
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

    // ACTION_DOWN is 0: Instant speech cancellation on touch down!
    if action == 0 {
        state.coordinator.lock().handle_event(AccessibilityEvent::SpeechInterrupt);
    }

    let result = state
        .touch_machine
        .lock()
        .process_touch(action, x as f64, y as f64, event_time);

    match result {
        TouchResult::Explore { x, y } => {
            let cache = state.node_cache.lock();
            if let Some(node) = cache.hit_test(x, y) {
                let accessible_node = AccessibleNode {
                    id: node.id,
                    role: node.role,
                    states: node.states,
                    name: if node.label.is_empty() { None } else { Some(node.label.clone()) },
                    bounds: Some(node.bounds),
                    ..Default::default()
                };
                state.last_focused_id.store(node.id.0 as i64, std::sync::atomic::Ordering::Relaxed);
                drop(cache);
                state.coordinator.lock().handle_event(AccessibilityEvent::Focus(accessible_node));
            }
            JNI_TRUE
        }
        TouchResult::Tap { x, y } => {
            let cache = state.node_cache.lock();
            if let Some(node) = cache.hit_test(x, y) {
                let accessible_node = AccessibleNode {
                    id: node.id,
                    role: node.role,
                    states: node.states,
                    name: if node.label.is_empty() { None } else { Some(node.label.clone()) },
                    bounds: Some(node.bounds),
                    ..Default::default()
                };
                state.last_focused_id.store(node.id.0 as i64, std::sync::atomic::Ordering::Relaxed);
                drop(cache);
                state.coordinator.lock().handle_event(AccessibilityEvent::Focus(accessible_node));
            }
            JNI_TRUE
        }
        TouchResult::DoubleTap { .. } => {
            log::info!("Double tap activated: performing AccessibleAction::Click");
            let mut coord = state.coordinator.lock();
            let _ = coord.execute_action(None, AccessibleAction::Click);
            JNI_TRUE
        }
        TouchResult::Flick(SwipeDirection::Right) => {
            log::info!("Swipe Right: advancing to next node in reading order");
            let current_id = state.last_focused_id.load(std::sync::atomic::Ordering::Relaxed) as u64;
            let cache = state.node_cache.lock();
            if let Some(next) = cache.next_node(current_id) {
                let accessible_node = AccessibleNode {
                    id: next.id,
                    role: next.role,
                    states: next.states,
                    name: if next.label.is_empty() { None } else { Some(next.label.clone()) },
                    bounds: Some(next.bounds),
                    ..Default::default()
                };
                state.last_focused_id.store(next.id.0 as i64, std::sync::atomic::Ordering::Relaxed);
                drop(cache);
                state.coordinator.lock().handle_event(AccessibilityEvent::Focus(accessible_node));
            }
            JNI_TRUE
        }
        TouchResult::Flick(SwipeDirection::Left) => {
            log::info!("Swipe Left: backtracking to previous node in reading order");
            let current_id = state.last_focused_id.load(std::sync::atomic::Ordering::Relaxed) as u64;
            let cache = state.node_cache.lock();
            if let Some(prev) = cache.previous_node(current_id) {
                let accessible_node = AccessibleNode {
                    id: prev.id,
                    role: prev.role,
                    states: prev.states,
                    name: if prev.label.is_empty() { None } else { Some(prev.label.clone()) },
                    bounds: Some(prev.bounds),
                    ..Default::default()
                };
                state.last_focused_id.store(prev.id.0 as i64, std::sync::atomic::Ordering::Relaxed);
                drop(cache);
                state.coordinator.lock().handle_event(AccessibilityEvent::Focus(accessible_node));
            }
            JNI_TRUE
        }
        TouchResult::Flick(_) => JNI_TRUE,
        TouchResult::None => JNI_FALSE,
    }
}
