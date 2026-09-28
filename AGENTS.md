# Agent & Contributor Guidelines for `bit_sr`

Welcome to `bit_sr`. This document specifies the architectural rules, coding invariants, safety protocols, and testing practices that all human contributors and AI agents must follow when developing this codebase.

---

## 1. Architectural Guardrails & Invariants

### Invariant 1: Zero Platform Code in Core and Engine
* **`bit_sr_core`** and **`bit_sr_engine`** must remain **100% platform-agnostic**.
* Never import Win32 (`windows`, `windows-sys`), COM, or Linux (`atspi`, `dbus`, `evdev`) APIs into `bit_sr_core` or the core engine logic.
* All platform interactions must flow through:
  - The unified [`AccessibilityEvent`](crates/bit_sr_core/src/events.rs) channel.
  - Pluggable driver traits ([`SynthesizerDriver`](crates/bit_sr_speech/src/synthesizer.rs), [`ActionPerformer`](crates/bit_sr_core/src/actions.rs)).

### Invariant 2: RAII COM Apartment Lifetime Management
* When initializing COM via `CoInitializeEx(None, COINIT_MULTITHREADED)`, **never** assign the guard to `let _ = ComGuard::init_mta()`. Doing so drops the guard immediately at statement end, calling `CoUninitialize()` and causing subsequent COM calls to fail with `0x800401F0` (`CO_E_NOTINITIALIZED`).
* Always store the COM guard as a named struct field (e.g. `_com_guard: ComInitGuard`) for the full duration of the object's lifecycle.

### Invariant 3: Windows UIA Root Subtree Restriction
* **Never** call `FindAll`, `FindAllBuildCache`, or `AddPropertyChangedEventHandlerNativeArray` on the Windows Desktop root element (`#32769`) with `TreeScope_Subtree`.
* Windows UI Automation explicitly prohibits global desktop subtree queries with error `0x8007029C` (`ERROR_ASSERTION_FAILURE`) to prevent system-wide lockups.
* Global desktop queries must strictly use `TreeScope_Children` (depth 1). Subtree queries must only be anchored to specific active top-level application windows.

### Invariant 4: Low-Level Hook Callback Safety
* The callback procedure for `WH_KEYBOARD_LL` (and `WH_MOUSE_LL`) runs on the Windows UI thread handling input.
* **NEVER** execute COM calls, heap-heavy allocations, or blocking locks inside the hook callback.
* The hook must only do lightweight bitwise filtering and call `try_send` on a lock-free crossbeam channel.

### Invariant 5: Instant Keystroke Interruption
* Screen reader responsiveness requires that typing or navigation immediately stops previous speech.
* On any physical key down that is not a pure modifier, the keyboard hook must dispatch `AccessibilityEvent::SpeechInterrupt`.
* All speech drivers must implement immediate audio purge (e.g., `(SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0) as u32` in SAPI 5).

### Invariant 6: Dynamic Live Window Title Tracking
* Do not rely solely on asynchronous `EVENT_SYSTEM_FOREGROUND` events to know the active window title.
* When the user requests `Insert + T` (Announce Title), query `get_foreground_window_title()` directly via Win32 `GetForegroundWindow()` and `GetWindowTextW()`.

---

## 2. Workspace Structure & Crate Ownership

```
crates/
├── bit_sr_core/               # Pure data models, Android-inspired tree, actions, roles, states
├── bit_sr_platform_windows/   # Win32 WH_KEYBOARD_LL, UIA CUIAutomation8, MSAA, Explorer heuristics
├── bit_sr_speech/             # Pluggable SpeechHub, SAPI 5 ISpVoice, MockSynthesizer
└── bit_sr_engine/             # Coordinator event loop, SpeechFormatter, FocusTracker, bit_sr CLI binary
```

### Dependency Flow
* `bit_sr_engine` depends on `bit_sr_core`, `bit_sr_speech`, and `bit_sr_platform_windows` (on Windows).
* `bit_sr_platform_windows` depends on `bit_sr_core`.
* `bit_sr_speech` depends on `bit_sr_core`.
* `bit_sr_core` has **no dependencies** on other workspace crates.

---

## 3. Coding Standards & Invariants

1. **Rust Edition:** Edition 2024. Use modern idioms (pattern matching, RAII guards, explicit error handling).
2. **Zero Compiler Warnings:** All code must compile with `cargo check` and `cargo test` with **zero warnings** and zero errors.
3. **Explicit Error Handling:** Do not use `unwrap()` in production paths. Propagate errors via `Result<T, Error>` or log them cleanly.
4. **Unit Tests Required:** Every new feature, state transition, and formatting rule must have corresponding unit tests.
5. **Deduplication:** When handling focus and state changes, apply anti-chatter deduplication in [`FocusTracker`](crates/bit_sr_engine/src/tracker.rs) to avoid redundant speech.

---

## 4. Verification Workflow

Before committing any changes:

```powershell
# 1. Run all workspace unit tests (must pass with 0 failures)
cargo test --workspace

# 2. Check all examples and binaries compile with 0 warnings
cargo check --workspace --examples --bins

# 3. Test the live prototype
cargo run --bin bit_sr
```

---

## 5. Git Commit Guidelines

Follow Conventional Commits:
* `feat(...)`: New features or drivers
* `fix(...)`: Bug fixes and edge case resolutions
* `perf(...)`: Latency and memory optimizations
* `docs(...)`: Documentation updates
* `refactor(...)`: Internal code improvements without behavioral changes
