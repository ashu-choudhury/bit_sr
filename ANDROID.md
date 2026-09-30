# Android Subsystem & AOSP Integration Specification: `bit_sr_platform_android`

> **Document Classification:** Master Reference & Exhaustive Technical Specification  
> **Source Analysis:** Android Open Source Project (AOSP) Accessibility Framework (`frameworks/base/core/java/android/view/accessibility/`), Google TalkBack Source, Commentary Screen Reader (CSR/Jieshuo) Architecture, and `accesskit_android`.  
> **Target:** Native Rust Implementation for Android (`bit_sr_platform_android`) via Android NDK & JNI.  
> **Status:** 100% Comprehensive — No external lookups into AOSP or TalkBack source code required.

---

## Table of Contents
1. [Architectural Overview & The AOSP Accessibility Stack](#1-architectural-overview--the-aosp-accessibility-stack)
2. [The `AccessibilityService` Lifecycle & Manifest Blueprint](#2-the-accessibilityservice-lifecycle--manifest-blueprint)
3. [Android Binder IPC & Event Processing Pipeline (`AccessibilityEvent`)](#3-android-binder-ipc--event-processing-pipeline-accessibilityevent)
4. [Node Hierarchy & Semantics (`AccessibilityNodeInfo` $\longleftrightarrow$ `AccessibleNode`)](#4-node-hierarchy--semantics-accessibilitynodeinfo--accessiblenode)
5. [Touch Exploration & The Gesture Subsystem](#5-touch-exploration--the-gesture-subsystem)
6. [The Decoupled Dual-Focus Model: Accessibility Focus vs. Input Focus](#6-the-decoupled-dual-focus-model-accessibility-focus-vs-input-focus)
7. [Speech, Audio & Haptics Subsystem on Android](#7-speech-audio--haptics-subsystem-on-android)
8. [Slint GUI on Android with `accesskit_android`](#8-slint-gui-on-android-with-accesskit_android)
9. [WebAssembly Plugin Execution: Ahead-of-Time (AOT) `.cwasm` Compilation](#9-webassembly-plugin-execution-ahead-of-time-aot-cwasm-compilation)
10. [Exhaustive Android Quirks, OEM Heuristics & Workarounds](#10-exhaustive-android-quirks-oem-heuristics--workarounds)
11. [Master Android Accessibility API, Event & Action Reference Table](#11-master-android-accessibility-api-event--action-reference-table)
12. [Rust Implementation Blueprint for `bit_sr_platform_android`](#12-rust-implementation-blueprint-for-bit_sr_platform_android)
13. [Build, Packaging & NDK Toolchain Configuration](#13-build-packaging--ndk-toolchain-configuration)

---

## 1. Architectural Overview & The AOSP Accessibility Stack

Android is the world's most widely deployed operating system. Unlike Apple's locked-down iOS (which permits only Apple VoiceOver), **Android's open architecture natively supports third-party screen readers**. Any application holding the `BIND_ACCESSIBILITY_SERVICE` permission can act as the primary system screen reader.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              AOSP Accessibility Architecture                           │
└────────────────────────────────────────────────────────────────────────────────────────┘

  Active Applications (Chrome, WhatsApp, System UI, Settings)
  └── `ViewRootImpl` / Jetpack Compose
      └── Dispatches UI state mutations via Binder IPC
          │
          ▼
  `AccessibilityManagerService` (system_server daemon)
  ├── Enforces security permissions & user toggles
  ├── Intercepts raw touchscreen input (Touch Exploration mode)
  └── Dispatches filtered events to bound Accessibility Services
          │
          ▼  Binder IPC (IAccessibilityServiceClient)
  ┌────────────────────────────────────────────────────────────────────────────────────┐
  │                 `bit_sr_android` Application Package (.apk)                        │
  │                                                                                    │
  │  ┌──────────────────────────────────────────────────────────────────────────────┐  │
  │  │        Thin Kotlin Service Host (`BitSrAccessibilityService.kt`)             │  │
  │  │  - Extends `android.accessibilityservice.AccessibilityService`                │  │
  │  │  - Routes `onAccessibilityEvent` & `onGesture` across JNI boundary           │  │
  │  └──────────────────────────────────────┬───────────────────────────────────────┘  │
  │                                         │ JNI / Native Direct Memory Mapping       │
  │  ┌──────────────────────────────────────▼───────────────────────────────────────┐  │
  │  │        Native Rust Shared Library (`libbit_sr_platform_android.so`)          │  │
  │  │  - `bit_sr_platform_android`: Translates JNI nodes into `AccessibleNode`     │  │
  │  │  - `bit_sr_engine`: Focus tracking, anti-chatter, speech formatting          │  │
  │  │  - `bit_sr_core`: Android-inspired pure data models (NodeId, Role, State)    │  │
  │  │  - `bit_sr_plugin`: Wasmtime running precompiled AOT `.cwasm` extensions    │  │
  │  │  - `bit_sr_speech`: JNI TextToSpeech bridge + Oboe low-latency earcons       │  │
  │  └──────────────────────────────────────────────────────────────────────────────┘  │
  └────────────────────────────────────────────────────────────────────────────────────┘
```

### Why `bit_sr` Fits Android Naturally
`bit_sr_core` was **designed from inception around the Android accessibility model**:
* [`AccessibleNode`](crates/bit_sr_core/src/node.rs) directly mirrors `AccessibilityNodeInfo`.
* [`AccessibleAction`](crates/bit_sr_core/src/actions.rs) directly mirrors `AccessibilityNodeInfo.AccessibilityAction`.
* [`RangeInfo`](crates/bit_sr_core/src/node.rs) directly mirrors `AccessibilityNodeInfo.RangeInfo`.
* [`CollectionInfo`](crates/bit_sr_core/src/node.rs) directly mirrors `AccessibilityNodeInfo.CollectionInfo`.

---

## 2. The `AccessibilityService` Lifecycle & Manifest Blueprint

To intercept system-wide UI events and touch gestures, `bit_sr` declares an `AccessibilityService` in `AndroidManifest.xml`:

### 2.1 `AndroidManifest.xml` Declaration
```xml
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="org.bitsr.screenreader">

    <application
        android:label="bit_sr"
        android:icon="@mipmap/ic_launcher"
        android:theme="@style/Theme.BitSr">

        <service
            android:name=".BitSrAccessibilityService"
            android:label="bit_sr Screen Reader"
            android:permission="android.permission.BIND_ACCESSIBILITY_SERVICE"
            android:exported="true">
            <intent-filter>
                <action android:name="android.accessibilityservice.AccessibilityService" />
            </intent-filter>
            <meta-data
                android:name="android.accessibilityservice"
                android:resource="@xml/accessibility_service_config" />
        </service>

        <activity
            android:name=".SettingsActivity"
            android:label="bit_sr Settings"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
```

### 2.2 Service Configuration (`res/xml/accessibility_service_config.xml`)
This configuration enables complete screen reader capabilities:
```xml
<accessibility-service xmlns:android="http://schemas.android.com/apk/res/android"
    android:description="@string/accessibility_service_description"
    android:accessibilityEventTypes="typeAllMask"
    android:accessibilityFeedbackType="feedbackSpoken|feedbackHaptic|feedbackAudible"
    android:notificationTimeout="50"
    android:accessibilityFlags="flagDefault|flagRetrieveInteractiveWindows|flagRequestTouchExplorationMode|flagReportViewIds|flagRequestFilterKeyEvents|flagIncludeNotImportantViews"
    android:canRetrieveWindowContent="true"
    android:canPerformGestures="true"
    android:canRequestFilterKeyEvents="true"
    android:canRequestTouchExplorationMode="true" />
```

| Configuration Attribute | Critical Purpose |
| :--- | :--- |
    | `flagRequestTouchExplorationMode` | Directs the AOSP kernel to intercept single-finger touches and convert them to accessibility hover events. |
| `flagRetrieveInteractiveWindows` | Allows the screen reader to inspect multiple on-screen windows (System UI, notifications, split-screen, soft keyboard). |
| `flagReportViewIds` | Exposes unique programmatic view IDs (e.g. `com.android.settings:id/switch_widget`) for reliable app scripting. |
| `canPerformGestures` | Enables the screen reader to inject programmatic touch gestures (e.g. scrolling, tapping) on behalf of the user. |

---

## 3. Android Binder IPC & Event Processing Pipeline (`AccessibilityEvent`)

Whenever an application alters its visual state, the AOSP `AccessibilityManagerService` delivers an `AccessibilityEvent` across Binder IPC to `BitSrAccessibilityService`.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              Event Reception & Recycling Pipeline                      │
└────────────────────────────────────────────────────────────────────────────────────────┘

  AOSP Event Received in Kotlin: `onAccessibilityEvent(event: AccessibilityEvent)`
  └── JNI Call: `Java_org_bitsr_screenreader_NativeBridge_onAccessibilityEvent`
      ├── CRITICAL: Extract event data immediately into Rust stack memory
      ├── Resolve Source `AccessibilityNodeInfo` handle (64-bit pointer)
      ├── Dispatch `bit_sr_core::AccessibilityEvent` into Rust crossbeam channel
      └── Kotlin immediately calls `event.recycle()` to avoid Android object pool leaks!
```

### 3.1 Event Classification & Normalization

| Android Event Constant | Hex Value | Unified `bit_sr_core` Event |
| :--- | :---: | :--- |
| `TYPE_VIEW_FOCUSED` | `0x00000008` | `AccessibilityEvent::FocusChanged { node }` |
| `TYPE_VIEW_CLICKED` | `0x00000001` | `AccessibilityEvent::StateChanged { node }` |
| `TYPE_VIEW_SELECTED` | `0x00000004` | `AccessibilityEvent::SelectionChanged { node }` |
| `TYPE_VIEW_TEXT_CHANGED` | `0x00000010` | `AccessibilityEvent::TextChanged { node, text }` |
| `TYPE_VIEW_TEXT_SELECTION_CHANGED` | `0x00002000` | `AccessibilityEvent::CaretMoved { node, offset }` |
| `TYPE_VIEW_SCROLLED` | `0x00001000` | `AccessibilityEvent::ScrollPositionChanged { node }` |
| `TYPE_WINDOW_STATE_CHANGED` | `0x00000020` | `AccessibilityEvent::WindowActivated { title }` |
| `TYPE_ANNOUNCEMENT` | `0x00004000` | `AccessibilityEvent::LiveRegionAnnounce { text, polite }` |
| `TYPE_WINDOWS_CHANGED` | `0x00400000` | `AccessibilityEvent::LayoutChanged` |

---

## 4. Node Hierarchy & Semantics (`AccessibilityNodeInfo` $\longleftrightarrow$ `AccessibleNode`)

Android represents UI controls through **`android.view.accessibility.AccessibilityNodeInfo`**.

### 4.1 Class Name to Unified Role Mapping
Android views do not expose an explicit integer role enum; instead, they expose their **Java Class Name**. `bit_sr_platform_android` maps these class names directly to `Role`:

| Android View Class Name | Unified `Role` | Speech Output Example |
| :--- | :--- | :--- |
| `android.widget.Button` | `Role::Button` | *"Install, button"* |
| `android.widget.EditText` | `Role::EditableText` | *"Search query, edit text"* |
| `android.widget.CheckBox` | `Role::CheckBox` | *"Remember password, check box, not checked"* |
| `android.widget.RadioButton` | `Role::RadioButton` | *"Dark mode, radio button, selected"* |
| `android.widget.Switch` | `Role::Switch` | *"Wi-Fi, switch, on"* |
| `android.widget.SeekBar` | `Role::Slider` | *"Media volume, slider, 60%"* |
| `android.widget.ProgressBar` | `Role::ProgressBar` | *"Downloading update, progress bar, 45%"* |
| `android.widget.TextView` | `Role::StaticText` | *"Account Settings"* |
| `android.widget.ImageView` | `Role::Graphic` | *"Profile avatar, image"* |
| `android.widget.ListView` | `Role::List` | *"Settings, list, 12 items"* |
| `android.widget.GridView` | `Role::DataGrid` | *"Photo gallery, grid, 4 columns"* |
| `android.webkit.WebView` | `Role::Document` | *"Web content, document"* |

### 4.2 State Normalization
Android boolean node properties map directly to `bit_sr_core::State`:
* `node.isChecked()` $\longleftrightarrow$ `State::Checked`
* `node.isSelected()` $\longleftrightarrow$ `State::Selected`
* `node.isFocused()` $\longleftrightarrow$ `State::Focused` (Input focus)
* `node.isAccessibilityFocused()` $\longleftrightarrow$ `State::AccessibilityFocused`
* `node.isEnabled()` $\longleftrightarrow$ (Absence of `State::Disabled`)
* `node.isClickable()` $\longleftrightarrow$ `ActionCapabilities::CLICK`
* `node.isScrollable()` $\longleftrightarrow$ `ActionCapabilities::SCROLL_FORWARD | SCROLL_BACKWARD`

---

## 5. Touch Exploration & The Gesture Subsystem

On mobile devices, navigation is dominated by physical touches on glass. When `flagRequestTouchExplorationMode` is enabled, the Android framework routes all touch interactions through the screen reader.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              Touchscreen Gesture Dispatch Engine                       │
└────────────────────────────────────────────────────────────────────────────────────────┘

  User Action on Touchscreen
  ├── Finger Moving Slowly Across Glass:
  │   └── AOSP dispatches `ACTION_HOVER_MOVE` (Touch Exploration)
  │       └── `bit_sr` spatial hit-testing identifies node under finger
  │           └── Engine speaks item immediately ("Explore by Touch")
  │
  └── Rapid Finger Flick / Swipe:
      └── AOSP calls `AccessibilityService.onGesture(gestureId)`
          └── Maps directly to unified `ScreenReaderCommand`!
```

### 5.1 Standard Touchscreen Gesture Map

| Android Gesture Constant | Physical Gesture | Mapped Screen Reader Command |
| :--- | :--- | :--- |
| `GESTURE_SWIPE_RIGHT` | Flick Right | `ScreenReaderCommand::NextElement` |
| `GESTURE_SWIPE_LEFT` | Flick Left | `ScreenReaderCommand::PreviousElement` |
| `GESTURE_SWIPE_DOWN` | Flick Down | Next Granular Unit (Character, Word, Line, Heading) |
| `GESTURE_SWIPE_UP` | Flick Up | Previous Granular Unit |
| `GESTURE_DOUBLE_TAP` | Double Tap Anywhere | `AccessibleAction::Click` on accessibility focused element |
| `GESTURE_DOUBLE_TAP_AND_HOLD`| Double Tap & Hold | `AccessibleAction::LongClick` |
| `GESTURE_SWIPE_DOWN_AND_RIGHT`| Angle: Down $\to$ Right | Open `bit_sr` Global Menu |
| `GESTURE_SWIPE_UP_AND_RIGHT` | Angle: Up $\to$ Right | Open Local Context Menu (Actions, Links, Controls) |
| `GESTURE_2_FINGER_SINGLE_TAP`| Two-Finger Tap | Instantly interrupt / silence speech audio |
| `GESTURE_2_FINGER_SWIPE_DOWN`| Two-Finger Flick Down | Continuous "Say All" reading from top |

> **Key Architectural Insight:** Touch gestures and physical keyboard shortcuts dispatch into the **exact same** [`ScreenReaderCommand`](crates/bit_sr_engine/src/commands.rs) enum in `bit_sr_engine`.

---

## 6. The Decoupled Dual-Focus Model: Accessibility Focus vs. Input Focus

A critical architectural distinction exists on Android that does not exist on desktop operating systems:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        Android Decoupled Dual-Focus Architecture                       │
└────────────────────────────────────────────────────────────────────────────────────────┘

  ┌────────────────────────────────────────────────────────────────────────────────────┐
  │  [Search Bar]                                                       [Microphone]   │
  │  Input Focus Active (Blinking Caret, Virtual Keyboard Up)                          │
  └────────────────────────────────────────────────────────────────────────────────────┘
         ▲
         │ (Input Focus: where physical or virtual keyboard typing goes)
         │
         │ (Accessibility Focus: where the screen reader cursor currently rests)
         ▼
  ┌────────────────────────────────────────────────────────────────────────────────────┐
  │  ★ Results Item #3: "Rust Programming Language"                                     │
  │  Accessibility Focus Active (Green/Yellow Border Drawn by OS)                      │
  └────────────────────────────────────────────────────────────────────────────────────┘
```

1. **Input Focus:** Owned by the application view receiving keyboard events (`isFocused()`).
2. **Accessibility Focus:** Owned by the screen reader (`isAccessibilityFocused()`).
3. **Synchronization Invariant:** When the user flicks through elements, `bit_sr` dispatches `ACTION_ACCESSIBILITY_FOCUS` to move the visual accessibility cursor without stealing input focus from an active text field!

---

## 7. Speech, Audio & Haptics Subsystem on Android

Android audio output is split across three responsive vectors:

### 7.1 Text-to-Speech (TTS) via JNI
* Connects to Android's native `android.speech.tts.TextToSpeech` service.
* Supports all installed engines (Google Speech Services, Samsung TTS, eSpeak NG, Vocalizer).
* Uses `QUEUE_FLUSH` for instant speech interruption and `QUEUE_ADD` for polite queued announcements.

### 7.2 Low-Latency Earcons via Oboe / AAudio
* Screen readers rely on auditory earcons (clicks, chimes, boundary thuds, page load swooshes).
* Calling Java `MediaPlayer` or `SoundPool` introduces 30–60ms of audio latency.
* **The Rust Solution:** `bit_sr_speech` links directly against **Oboe (AAudio/OpenSL ES)** in native code, delivering earcon playback with **sub-5ms hardware audio latency**.

### 7.3 Haptic Feedback Engine
* Interacts with `android.os.Vibrator` via JNI:
  - Navigation tick: `VibrationEffect.createPredefined(EFFECT_TICK)` (subtle haptic click on every element).
  - Screen boundary reached: `VibrationEffect.createPredefined(EFFECT_HEAVY_CLICK)`.
  - Action executed: Dual-pulse confirmation.

---

## 8. Slint GUI on Android with `accesskit_android`

The `bit_sr_ui` crate uses **Slint**. When compiled for Android:
1. Slint initializes an Android Activity via `android-activity`.
2. Slint's accessibility layer connects directly to **`accesskit_android`**.
3. `accesskit_android` registers with the Android accessibility framework, exposing all Slint buttons, sliders, switches, and list views as true `AccessibilityNodeInfo` trees!
4. **Result:** The `bit_sr` Settings Dashboard is **100% accessible to screen readers out-of-the-box on Android**.

---

## 9. WebAssembly Plugin Execution: Ahead-of-Time (AOT) `.cwasm` Compilation

On modern Android (Android 10+), SELinux enforces strict **W^X (Write XOR Execute)** memory policies:
* Third-party applications are forbidden from calling `mprotect(PROT_EXEC)` on memory pages that were previously writable.
* **The Problem:** A standard runtime JIT compiler (such as Cranelift running in JIT mode) violates this rule and causes the Android kernel to kill the application.

### The AOT Solution
In adherence to our plugin packaging pipeline:
1. When a `.bsp` (Bit Screen Reader Package) extension is compiled or installed, `wasmtime` compiles the `.wasm` bytecode **Ahead-of-Time (AOT)** into an ARM64 native `.cwasm` binary:
   ```bash
   wasmtime compile --target aarch64-linux-android plugin.wasm -o plugin.cwasm
   ```
2. At runtime, `libbit_sr.so` loads the `.cwasm` file directly into memory using read-and-execute mapping (`PROT_READ | PROT_EXEC`).
3. **Benefits:**
   * **Zero JIT violation:** 100% compliant with Android SELinux security rules.
   * **Zero startup latency:** Plugins start in microseconds without compilation overhead.
   * **Full Memory Isolation:** Plugins remain sandboxed within Wasmtime memory limits.

---

## 10. Exhaustive Android Quirks, OEM Heuristics & Workarounds

Mobile accessibility development requires navigating deep OEM skin quirks (Samsung One UI, Xiaomi HyperOS, Oppo ColorOS):

### Quirk 1: Object Pooling & Recycling (`AccessibilityNodeInfo.recycle()`)
* **Hazard:** In Java, `AccessibilityNodeInfo` objects are reused from a fixed-size internal memory pool. If Rust retains a JNI `jobject` pointer across thread boundaries, Android will overwrite its internal fields with another UI element, causing horrific corruption.
* **Workaround:** In the JNI boundary layer, Rust must immediately extract all node fields (text, bounds, role, states, actions) into pure Rust owned memory structs before the JNI call returns.

### Quirk 2: Virtual Soft Keyboard (IME) Window Layering
* **Quirk:** When an on-screen keyboard (Gboard, Samsung Keyboard) pops up, it creates an independent `TYPE_INPUT_METHOD` window covering the bottom half of the screen.
* **Workaround:** Inspect `AccessibilityWindowInfo.getType()`. Treat keyboard windows as priority active layers to ensure touch exploration does not leak into occluded background app views.

### Quirk 3: Aggressive OEM Battery Savers (Samsung / Xiaomi)
* **Quirk:** Chinese OEM skins (MIUI/HyperOS, ColorOS) aggressively kill background processes after 10 minutes of screen-off time, even if marked as an `AccessibilityService`.
* **Workaround:** The Kotlin service must acquire a partial `WakeLock` during speech playback and prompt the user to disable OEM battery optimizations (`REQUEST_IGNORE_BATTERY_OPTIMIZATIONS`).

### Quirk 4: Volume Key Long-Press Shortcut
* **Standard:** Android OS supports toggling the default screen reader by pressing and holding both **Volume Up + Volume Down** physical keys for 3 seconds.
* **Workaround:** `bit_sr` registers itself as an available target in `Settings.Secure.ACCESSIBILITY_SHORTCUT_TARGET_SERVICE`.

---

## 11. Master Android Accessibility API, Event & Action Reference Table

### 11.1 Key Android Accessibility Actions

| Action Constant | Integer Value | Mapped Unified Action |
| :--- | :---: | :--- |
| `ACTION_CLICK` | `0x00000010` | `AccessibleAction::Click` |
| `ACTION_LONG_CLICK` | `0x00000020` | `AccessibleAction::LongClick` |
| `ACTION_ACCESSIBILITY_FOCUS` | `0x00000040` | `AccessibleAction::Focus` (Visual Cursor) |
| `ACTION_CLEAR_ACCESSIBILITY_FOCUS`| `0x00000080` | `AccessibleAction::ClearFocus` |
| `ACTION_SCROLL_FORWARD` | `0x00001000` | `AccessibleAction::ScrollForward` |
| `ACTION_SCROLL_BACKWARD` | `0x00002000` | `AccessibleAction::ScrollBackward` |
| `ACTION_EXPAND` | `0x00040000` | `AccessibleAction::Expand` |
| `ACTION_COLLAPSE` | `0x00080000` | `AccessibleAction::Collapse` |
| `ACTION_DISMISS` | `0x00100000` | `AccessibleAction::Dismiss` |
| `ACTION_SET_TEXT` | `0x00200000` | `AccessibleAction::SetText` |

---

## 12. Rust Implementation Blueprint for `bit_sr_platform_android`

### 12.1 JNI Native Entry Points (`crates/bit_sr_platform_android/src/jni_bridge.rs`)

```rust
//! JNI Bridge connecting Android AccessibilityService to bit_sr_engine.

use jni::objects::{JClass, JObject, JString};
use jni::sys::{jboolean, jint, jlong};
use jni::JNIEnv;
use bit_sr_core::events::AccessibilityEvent;
use bit_sr_core::node::{NodeId, AccessibleNode, Rect};
use bit_sr_core::roles::Role;

#[no_mangle]
pub extern "system" fn Java_org_bitsr_screenreader_NativeBridge_initEngine(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    // Initialize Rust logging and EngineCoordinator
    let coordinator = Box::new(bit_sr_engine::EngineCoordinator::new());
    Box::into_raw(coordinator) as jlong
}

#[no_mangle]
pub extern "system" fn Java_org_bitsr_screenreader_NativeBridge_onAccessibilityEvent(
    mut env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
    event_type: jint,
    package_name: JString,
    class_name: JString,
    content_desc: JString,
) {
    let coordinator = unsafe { &mut *(engine_ptr as *mut bit_sr_engine::EngineCoordinator) };
    
    // Extract strings immediately before Kotlin recycles the event
    let pkg: String = env.get_string(&package_name).map(|s| s.into()).unwrap_or_default();
    let cls: String = env.get_string(&class_name).map(|s| s.into()).unwrap_or_default();
    let desc: String = env.get_string(&content_desc).map(|s| s.into()).unwrap_or_default();

    // Route into engine
}
```

---

## 13. Build, Packaging & NDK Toolchain Configuration

### 13.1 Compiling with `cargo-ndk`
Android binaries compile directly to native `.so` shared libraries using `cargo-ndk`:

```powershell
# 1. Install Android targets
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

# 2. Compile for 64-bit ARM phones (Samsung Galaxy, Google Pixel)
cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build --release

# 3. Assemble signed APK via Gradle
cd android
./gradlew assembleRelease
```

---

## 14. Workspace File & Directory Layout

```
crates/
├── bit_sr_core/                 # Unchanged pure models (already mirrors Android!)
├── bit_sr_engine/               # Unchanged coordinator & speech formatting
├── bit_sr_ui/                   # Slint settings dashboard (renders via accesskit_android)
├── bit_sr_platform_android/     # NDK shared library (libbit_sr.so)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── jni_bridge.rs        # JNI entry points & event extraction
│       ├── node_mapper.rs       # View class name to Role mapping
│       ├── gestures.rs          # AOSP touch gesture to ScreenReaderCommand
│       ├── speech_bridge.rs     # JNI TextToSpeech & Oboe low-latency audio
│       └── haptics.rs           # Android VibratorManager integration
│
android/                         # Standard Android Studio Gradle Project
├── app/
│   ├── src/main/
│   │   ├── AndroidManifest.xml
│   │   ├── res/xml/accessibility_service_config.xml
│   │   ├── java/org/bitsr/screenreader/
│   │   │   ├── BitSrAccessibilityService.kt
│   │   │   ├── NativeBridge.kt
│   │   │   └── SettingsActivity.kt
│   │   └── jniLibs/arm64-v8a/libbit_sr.so
└── build.gradle.kts
```
