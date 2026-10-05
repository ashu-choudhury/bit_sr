# `bit_sr` Architectural Promise & Android Prototype Master Specification

> **Document Classification:** Binding Architectural Blueprint & Implementation Specification  
> **Document Name:** `PROMISE.md`  
> **Authority:** Project Architectural Vision & Core Roadmap  
> **Target:** Native Android First Prototype & Unified Engine Realignment  
> **Status:** Active & Normative — Mandates absolute compliance for all contributors and agents.

---

## Table of Contents
1. [The Architectural Promise & Vision Realignment](#1-the-architectural-promise--vision-realignment)
2. [Root Dependency Topology & Invariants](#2-root-dependency-topology--invariants)
3. [The Core Pillars on Android](#3-the-core-pillars-on-android)
   - [3.1 `bit_sr_engine`: The Universal Screen Reader Brain](#31-bit_sr_engine-the-universal-screen-reader-brain)
   - [3.2 `bit_sr_plugin`: Wasmtime AOT `.cwasm` Runtime](#32-bit_sr_plugin-wasmtime-aot-cwasm-runtime)
   - [3.3 `bit_sr_ui`: Slint with Native `accesskit_android`](#33-bit_sr_ui-slint-with-native-accesskit_android)
   - [3.4 `bit_sr_platform_android`: The Pure Hardware & OS Adapter](#34-bit_sr_platform_android-the-pure-hardware--os-adapter)
4. [Resolution of Past Architectural Anti-Patterns](#4-resolution-of-past-architectural-anti-patterns)
5. [JNI Boundary & Native Shared Library Architecture (`libbit_sr.so`)](#5-jni-boundary--native-shared-library-architecture-libbit_srso)
6. [Android First Prototype Implementation Roadmap](#6-android-first-prototype-implementation-roadmap)
   - [Phase 1: Engine Restructuring & Android Target Wiring](#phase-1-engine-restructuring--android-target-wiring)
   - [Phase 2: JNI Engine Coordination & Event Injection](#phase-2-jni-engine-coordination--event-injection)
   - [Phase 3: Text-to-Speech (TTS) & Earcon Audio Loop](#phase-3-text-to-speech-tts--earcon-audio-loop)
   - [Phase 4: Raw Touch Interception & Action Performer](#phase-4-raw-touch-interception--action-performer)
   - [Phase 5: Gradle Wrapper & NDK Packaging Pipeline](#phase-5-gradle-wrapper--ndk-packaging-pipeline)
   - [Phase 6: Prototype Verification & Cloud CI Integration](#phase-6-prototype-verification--cloud-ci-integration)
7. [Verification & Acceptance Criteria](#7-verification--acceptance-criteria)

---

## 1. The Architectural Promise & Vision Realignment

Screen readers have historically suffered from platform fragmentation, sluggish runtime interpreters, and uncontained plugin architectures. `bit_sr` was founded to eliminate these issues with a single, uncompromising vision:

1. **A Single Unified Screen Reader Brain (`bit_sr_engine`):**  
   The core intelligence of `bit_sr` — focus tracking, anti-chatter deduplication, speech formatting, gesture dispatching, command registries, and extension lifecycles — lives in **`bit_sr_engine`**. It is NOT a desktop-only binary; it is the universal engine for Windows, Linux, and Android.
2. **Platform Crates are Pure Adapters:**  
   Platform crates (`bit_sr_platform_windows`, `bit_sr_platform_android`, `bit_sr_platform_linux`) are strictly low-level Platform Abstraction Layers (PALs). They adapt OS-specific APIs, input hooks, and accessibility trees into the engine. They **never** duplicate engine logic, speech formatting, or focus tracking.
3. **Extensions are Universal & First-Class (`bit_sr_plugin`):**  
   WebAssembly extension support is a foundational requirement, not an optional desktop add-on. Every plugin is isolated in a capability-based **Wasmtime** sandbox. On Android, extensions run via precompiled **Ahead-of-Time (AOT) `.cwasm`** binaries, ensuring 0ms startup and zero SELinux `W^X` memory violations.
4. **Accessible Native UI Across All Screens (`bit_sr_ui`):**  
   The settings dashboard and extension manager are built with **Slint**. Slint compiles to both desktop and Android, integrating natively with `accesskit_android` to expose every control directly to the accessibility tree.

---

## 2. Root Dependency Topology & Invariants

```
                            ┌────────────────────────────────────────────────────────┐
                            │               bit_sr_core (Data Models)                │
                            │  (Role, State, NodeId, Rect, AccessibleAction, Event)  │
                            └───────────────────────────┬────────────────────────────┘
                                                        │
               ┌──────────────────┬─────────────────────┼────────────────────┬──────────────────┐
               │                  │                     │                    │                  │
        ┌──────▼──────┐    ┌──────▼──────┐       ┌──────▼──────┐      ┌──────▼──────┐    ┌──────▼──────┐
        │bit_sr_speech│    │bit_sr_plugin│       │  bit_sr_ui  │      │  Windows PAL│    │ Android PAL │
        │ (SpeechHub) │    │  (Wasmtime  │       │   (Slint +  │      │  (UIA/MSAA/ │    │ (Touch / R- │
        │             │    │ AOT .cwasm) │       │ AccessKit)  │      │   Hooks)    │    │ Tree Cache) │
        └──────┬──────┘    └──────┬──────┘       └──────┬──────┘      └──────┬──────┘    └──────┬──────┘
               │                  │                     │                    │                  │
               └──────────────────┴─────────────────────┼────────────────────┴──────────────────┘
                                                        │
                                                        ▼
                                 ┌─────────────────────────────────────────────┐
                                 │          bit_sr_engine (THE BRAIN)          │
                                 │                                             │
                                 │  • EngineCoordinator (Central Event Loop)   │
                                 │  • FocusTracker (Anti-Chatter Engine)       │
                                 │  • SpeechFormatter (i18n Speech Output)     │
                                 │  • PluginManager (Wasm Sandboxed Plugins)   │
                                 └──────────────────────┬──────────────────────┘
                                                        │
                                 ┌──────────────────────┴──────────────────────┐
                                 ▼                                             ▼
                         Desktop Artifact                              Android Artifact
                         `bit_sr` (Executable Binary)                  `libbit_sr.so` (cdylib Shared Lib)
                         - Win32 hooks / AT-SPI2                       - Loaded by BitSrAccessibilityService
                         - Direct SAPI/OneCore/speechd                 - NativeBridge JNI exports
```

### Architectural Invariants:
* **Invariant A (No Platform Logic in Engine):** `bit_sr_engine` never directly executes OS syscalls or imports raw Win32/AOSP APIs. All OS interactions flow through the platform adapters and driver traits.
* **Invariant B (Engine is the Top-Level Crate):** Platform crates depend on `bit_sr_core` and `bit_sr_speech`. The platform crates **never** depend on `bit_sr_engine`. `bit_sr_engine` depends on the platform crates conditionally (`cfg(windows)`, `cfg(target_os = "android")`).
* **Invariant C (Unified Output):** `bit_sr_engine` produces both:
  * The desktop CLI executable `bit_sr` (`[[bin]]`).
  * The native Android dynamic shared library `libbit_sr.so` (`[lib] crate-type = ["cdylib", "rlib"]`).

---

## 3. The Core Pillars on Android

### 3.1 `bit_sr_engine`: The Universal Screen Reader Brain
* Contains **`EngineCoordinator`**, the central coordinator that processes incoming events.
* Houses **`FocusTracker`**:
  * Tracks the active accessibility focus vs. input focus.
  * Enforces **anti-chatter deduplication** so rapid duplicate OS events (e.g. 5 focus events within 50ms) do not cause stuttering speech.
* Houses **`SpeechFormatter`**:
  * Formats accessible nodes (`Role`, `label`, `State`) into clean spoken strings (*"Settings, button"*, *"Wi-Fi, switch, on"*).
  * Backed by `LocalizationManager` for runtime multi-language support (English, Spanish, Hindi, German, French).
* Coordinates **`SpeechHub`**:
  * Dispatches formatted speech to the active synthesizer driver (`AndroidTtsDriver`) with prioritization (`Now` vs. `Next`).
  * Enforces immediate speech interruption on any user interaction.

### 3.2 `bit_sr_plugin`: Wasmtime AOT `.cwasm` Runtime
* Extensions on Android are **not** an optional feature; they are part of the core screen reader capabilities.
* Modern Android (API 29+) strictly enforces **W^X memory protection** via SELinux (`mprotect(PROT_EXEC)` is prohibited on writable pages).
* **The Solution:** Plugins are precompiled **Ahead-of-Time (AOT)** into native ARM64 `.cwasm` binaries during installation (`wasmtime compile --target aarch64-linux-android`).
* At runtime, `bit_sr_engine` loads `.cwasm` files directly into memory via `PROT_READ | PROT_EXEC` mapping, guaranteeing:
  * **Zero JIT SELinux violations.**
  * **Microsecond plugin startup.**
  * **Full memory sandboxing and epoch interruption.**

### 3.3 `bit_sr_ui`: Slint with Native `accesskit_android`
* The settings dashboard, left-dock menu, and extension storefront are implemented in `bit_sr_ui` using **Slint**.
* Slint compiles natively to Android via `android-activity`.
* Slint's accessibility layer connects directly to **`accesskit_android`**, exposing UI widgets as real Android `AccessibilityNodeInfo` elements.
* Blind users get a 100% accessible native configuration interface out of the box.

### 3.4 `bit_sr_platform_android`: The Pure Hardware & OS Adapter
Structured into clean, modular subsystems:
* **`tree/`:**
  * `cache.rs`: `SpatialNodeCache` maintaining in-memory bounding boxes of on-screen elements for zero-Binder hit testing.
  * `mapper.rs`: Translates Android View class names to `Role` and unpacks state bitmasks into `State` bitflags.
  * `node.rs`: `CachedNode` representation.
* **`input/`:**
  * `touch.rs`: `TouchStateMachine` handling raw `MotionEvent`s (`ACTION_DOWN`, `MOVE`, `UP`, `CANCEL`) to detect touch exploration, instant double-tap (<250ms), and directional flicks (`SwipeDirection`).
  * `keyboard.rs`: Hardware key tracker for physical Volume Up + Volume Down shortcut.
* **`feedback/`:**
  * `tts.rs`: `AndroidTtsDriver` implementing `SynthesizerDriver`.
  * `earcons.rs`: Low-latency audio cue identifiers (Tick, Boundary, Click, Scroll, PageLoad).
  * `haptics.rs`: Predefined haptic feedback codes (`EFFECT_TICK`, `EFFECT_CLICK`, `EFFECT_HEAVY_CLICK`).
* **`apps/`:**
  * Heuristics for Chrome/WebView, virtual soft keyboards (IME layering), and Android System UI bars.

---

## 4. Resolution of Past Architectural Anti-Patterns

| Anti-Pattern Identified | Root Cause | Architectural Resolution |
| :--- | :--- | :--- |
| **Platform Crate Inverted Dependency** | `bit_sr_platform_android` previously depended on `bit_sr_engine`. | Removed `bit_sr_engine` from platform crate. `bit_sr_engine` now conditionally depends on `bit_sr_platform_android` on Android. |
| **Duplicating Engine in Platform** | Attempting to write a second focus tracker / formatter inside Android crate. | Core logic stays strictly in `bit_sr_engine`. The platform crate only passes harvested nodes and coordinates up to the engine. |
| **Legacy AOSP Gesture Detector Baggage** | Retaining `from_aosp_id` mappings for slow AOSP `TouchExplorer` gestures. | Completely removed `gestures.rs`. Intercepting raw hardware `MotionEvent`s in Rust with zero OS delay timers. |
| **Separating Android from Plugin/UI Ecosystem** | Treating plugins and Slint UI as desktop-only modules. | Realigned `bit_sr_plugin` and `bit_sr_ui` as core dependencies across all targets, utilizing AOT `.cwasm` and `accesskit_android`. |

---

## 5. JNI Boundary & Native Shared Library Architecture (`libbit_sr.so`)

`bit_sr_engine` is compiled as a C-compatible dynamic shared library (`cdylib`) named **`bit_sr`**, generating **`libbit_sr.so`**.

```
Android OS (Zygote / ART Runtime)
└── `BitSrAccessibilityService.kt`
    └── Loads `System.loadLibrary("bit_sr")`
        │
        ├── JNI Call: NativeBridge.initEngine()
        │   └── Instantiates `EngineCoordinator`
        │       ├── Registers `AndroidTtsDriver` in `SpeechHub`
        │       ├── Starts `AndroidPlatform`
        │       ├── Initializes `PluginManager`
        │       └── Returns `jlong` (64-bit pointer to EngineCoordinator)
        │
        ├── JNI Call: NativeBridge.onAccessibilityEvent(...)
        │   └── Resolves `EngineCoordinator` pointer
        │       ├── Adapts event via `bit_sr_platform_android::tree::mapper`
        │       ├── `coordinator.handle_event(event)`
        │       ├── `FocusTracker` filters duplicates
        │       ├── `SpeechFormatter` formats speech ("Settings, button")
        │       └── `SpeechHub` speaks via `AndroidTtsDriver`
        │
        ├── JNI Call: NativeBridge.onRawTouch(...)
        │   └── Resolves `EngineCoordinator` pointer
        │       ├── Feeds raw coordinates to `TouchStateMachine`
        │       ├── Explore: Spatial hit test -> `coordinator.handle_event(FocusChanged)`
        │       ├── Double-Tap: `coordinator.execute_action(AccessibleAction::Click)`
        │       └── Flick: `coordinator.execute_command(command)`
        │
        └── JNI Call: NativeBridge.updateWindowTree(...)
            └── Flattens on-screen nodes into `SpatialNodeCache`
```

---

## 6. Android First Prototype Implementation Roadmap

### Phase 1: Engine Restructuring & Android Target Wiring
* [x] Reorganize `bit_sr_platform_android` into clean modular subsystems (`tree/`, `input/`, `feedback/`, `apps/`, `platform.rs`).
* [x] Eliminate obsolete AOSP `gestures.rs` and consolidate `SwipeDirection` into `touch.rs`.
* [x] Configure `crates/bit_sr_engine/Cargo.toml`:
  * Add `[lib] name = "bit_sr" crate-type = ["cdylib", "rlib"]`.
  * Add `[target.'cfg(target_os = "android")'.dependencies]` linking `bit_sr_platform_android` and `jni = "0.21"`.
* [x] Ensure `bit_sr_engine` compiles cleanly for both desktop and Android targets with zero compiler warnings.

### Phase 2: JNI Engine Coordination & Event Injection
* [x] Implement JNI exports in `bit_sr_engine` (e.g. `src/android_jni.rs` or unified module):
  * `Java_org_bitsr_screenreader_NativeBridge_initEngine`: Allocates and returns `EngineCoordinator`.
  * `Java_org_bitsr_screenreader_NativeBridge_destroyEngine`: Safely deallocates `EngineCoordinator`.
  * `Java_org_bitsr_screenreader_NativeBridge_onAccessibilityEvent`: Dispatches into `coordinator.handle_event()`.
  * `Java_org_bitsr_screenreader_NativeBridge_updateWindowTree`: Updates `SpatialNodeCache`.
  * `Java_org_bitsr_screenreader_NativeBridge_onRawTouch`: Processes touch and triggers engine actions.

### Phase 3: Text-to-Speech (TTS) & Earcon Audio Loop
* [x] Wire Android native `TextToSpeech` into `BitSrAccessibilityService.kt`:
  * Initialize `android.speech.tts.TextToSpeech` in service startup.
  * Provide native-to-Java callback `NativeBridge.speakText(text: String, interrupt: Boolean)` or direct JNI call from `AndroidTtsDriver`.
  * Use `TextToSpeech.QUEUE_FLUSH` on speech interruption for instant silence on key/touch down.
* [x] Verify spoken announcements for on-screen controls through `SpeechFormatter` (*"Home, button"*, *"Search, edit text"*).

### Phase 4: Raw Touch Interception & Action Performer
* [x] Connect `TouchInteractionController` (Android 13+ / API 33+) in `BitSrAccessibilityService.kt` to forward raw `MotionEvent`s to `NativeBridge.onRawTouch`.
* [x] Implement Action Execution:
  * On `TouchResult::DoubleTap`: Native engine dispatches `AccessibleAction::Click` $\to$ service executes `AccessibilityNodeInfo.performAction(ACTION_CLICK)` on the focused node.
  * On `TouchResult::Flick(SwipeDirection::Right)`: Native engine advances focus to the next node in linear reading order and speaks it.
  * On `TouchResult::Flick(SwipeDirection::Left)`: Backtracks focus to previous node and speaks it.

### Phase 5: Gradle Wrapper & NDK Packaging Pipeline
* [x] Provide Gradle wrapper scripts (`gradlew`, `gradlew.bat`, `gradle/wrapper/gradle-wrapper.jar`) in `android/`.
* [x] Create an automated build script (`scripts/build_android.ps1` / `scripts/build_android.sh`):
  * Compiles `libbit_sr.so` using `cargo ndk` for `arm64-v8a` (and `x86_64` for emulators).
  * Copies libraries into `android/app/src/main/jniLibs/`.
  * Invokes `./gradlew assembleDebug`.
  * Outputs standalone installable `bit_sr-debug.apk`.

### Phase 6: Prototype Verification & Cloud CI Integration
* [x] Install `bit_sr-debug.apk` onto an Android test device / emulator.
* [x] Enable `bit_sr Screen Reader` in Android Settings $\to$ Accessibility.
* [x] Validate explore-by-touch audio feedback, double-tap activation, and swipe navigation.
* [x] Update GitHub Actions workflow (`.github/workflows/ci.yml`) to automatically compile and release Android APK packages alongside Windows builds on every push to `master`.

---

## 7. Verification & Acceptance Criteria

Before declaring the first Android prototype complete, the following criteria must be met:

1. **Compilation Quality:**
   * `cargo test --workspace` passes with **0 failures**.
   * `cargo check --workspace --all-targets` compiles with **0 warnings and 0 errors**.
   * NDK target compilation (`cargo build --target aarch64-linux-android`) succeeds with **0 errors**.
2. **Audio & Feedback Latency:**
   * Touching any on-screen button speaks the label and role (*"Label, button"*) within **< 15ms**.
   * Touching the screen while previous speech is playing immediately **interrupts** previous speech with zero overlap.
3. **Touch Exploration & Gesture Control:**
   * Dragging a finger across the display speaks elements continuously via spatial hit-testing.
   * Double-tapping activates the focused control (`ACTION_CLICK`).
   * Swiping right/left navigates through elements in reading order.
4. **Standalone Packaging:**
   * A single, installable `bit_sr-debug.apk` is generated that runs out of the box without manual environment hacks.

---

*This document serves as our binding architectural contract and direct action plan for delivering the first Android prototype of `bit_sr`.*
