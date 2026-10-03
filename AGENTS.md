# Agent & Contributor Guidelines for `bit_sr`

Welcome to `bit_sr`. This document specifies the architectural rules, reference resources, coding invariants, safety protocols, and testing practices that all human contributors and AI agents must follow when developing this codebase.

---

## 1. Reference Codebases & Project Blueprints

### A. Upstream Reference Repositories (`references/`)
To inspect how mature screen readers interact with operating systems and accessibility APIs, `bit_sr` vendors upstream reference codebases as Git submodules in the `references/` directory:
* **NVDA (Windows reference):** [`references/nvda`](references/nvda) — Windows UIA, MSAA, IAccessible2, display model, and synthesizer handling.
* **Orca (Linux reference):** [`references/orca`](references/orca) — AT-SPI2 D-Bus protocol, speech-dispatcher, and GNOME/Nautilus integration.

When initializing or cloning the repository for the first time, always populate these reference submodules:
```powershell
git submodule update --init --recursive
```
> **Guideline:** Use these references to understand low-level OS quirks, heuristics, and behavior. Never copy GPL-licensed code directly into `bit_sr` (which is dual-licensed MIT/Apache-2.0). Implement clean-room Rust designs inspired by their behaviors.

### B. Root Architecture Blueprints
Before implementing or modifying platform-specific features, consult the self-contained technical specifications at the project root:
* **[`WINDOWS.md`](WINDOWS.md):** Complete Windows OS accessibility architecture & syscall blueprint (Win32 low-level hooks, UIA COM interfaces, event coalescing, cache requests, common controls, and File Explorer quirks).
* **[`LINUX.md`](LINUX.md):** Complete Linux AT-SPI2 D-Bus & evdev architecture blueprint (D-Bus interfaces, cache subtree batching, Wayland input capture, and GNOME Shell/Nautilus navigation).
* **[`CHROMIUM.md`](CHROMIUM.md):** Complete Chromium, Chrome, Microsoft Edge, Electron, and WebView2 accessibility architecture & virtual buffer blueprint (`AXMode`, Browse Mode vs. Focus Mode state machine, quick nav, ARIA live regions, and all 12 Blink heuristics).
* **[`FIREFOX.md`](FIREFOX.md):** Complete Mozilla Firefox, Tor Browser, and Gecko accessibility architecture blueprint (`accessible/` engine, "Cache the World" parent-process IPC, `IAccessible2`, `ISimpleDOMNode`, and anti-fingerprinting).
* **[`ANDROID.md`](ANDROID.md):** Complete Android OS & AOSP accessibility architecture blueprint (`AccessibilityService`, `AccessibilityNodeInfo` mapping, touch gestures, decoupled dual-focus, `accesskit_android`, and AOT `.cwasm`).
* **[`VISION.md`](VISION.md):** High-level roadmap, core philosophies, threat modeling, and overall project pillars.
* **[`VISION_WASM_EXTENSION.md`](VISION_WASM_EXTENSION.md):** Complete WebAssembly extension ecosystem specification (Wasmtime runtime, scoped storage, two-tier permission model, host gatekeeper, and multi-language support).

---

## 2. Architectural Guardrails & Invariants

### Invariant 1: Zero Platform Code in Core and Engine
* **`bit_sr_core`** and **`bit_sr_engine`** must remain **100% platform-agnostic**.
* Never import Win32 (`windows`, `windows-sys`), COM, or Linux (`atspi`, `dbus`, `evdev`) APIs into `bit_sr_core` or the core engine logic.
* All platform interactions must flow through:
  - The unified [`AccessibilityEvent`](crates/bit_sr_core/src/events.rs) channel.
  - Pluggable driver traits ([`SynthesizerDriver`](crates/bit_sr_speech/src/synthesizer.rs), [`ActionPerformer`](crates/bit_sr_core/src/actions.rs)).

### Invariant 2: RAII Resource & Apartment Lifetime Management
* Platform resources (such as Windows COM apartments, D-Bus connections, and audio hardware handles) must be managed using strict RAII guard structs.
* Never drop initialization guards at statement end (e.g. `let _ = ...`). Always store guards as named fields within the owning struct for the full duration of its lifecycle.

### Invariant 3: Focus-Anchored & Scoped Queries
* Never perform unbounded, full-system subtree crawls from the desktop root. Global queries across millions of application elements freeze operating systems and trigger protection aborts.
* Global queries must strictly inspect top-level window containers (depth 1). Deep subtree navigation must always be anchored to the active top-level application window or focused document.

### Invariant 4: Non-Blocking Low-Level Input Hooks
* The callback procedures for low-level input hooks (e.g., Windows `WH_KEYBOARD_LL`, Linux `evdev`) run directly on the UI input thread.
* **NEVER** execute COM calls, heap-heavy allocations, file I/O, or blocking locks inside the hook callback.
* Hook callbacks must only perform lightweight filtering and dispatch events via lock-free channels (`try_send`).

### Invariant 5: Instant Keystroke Interruption
* Screen reader responsiveness requires that typing or navigation immediately stops previous speech.
* On any physical key down that is not a pure modifier, the keyboard hook must dispatch `AccessibilityEvent::SpeechInterrupt`.
* All speech drivers must implement immediate audio purge (e.g., `SPF_PURGEBEFORESPEAK` in SAPI 5).

### Invariant 6: Synchronous Browse Mode Interception
* In web documents and virtual buffers, Browse Mode navigation and single-letter quick-nav keystrokes (`H`, `K`, `B`, `F`, `T`, `1`-`6`, arrows, `Space`, etc.) must be intercepted synchronously within the low-level keyboard hook callback (`WH_KEYBOARD_LL`) via atomic state inspection (`is_browse_mode_active()`).
* Keystrokes meant for screen reader buffer navigation must never leak into web browsers, web forms, or OS windows.

### Invariant 7: Document-Root Virtual Buffer Tree Harvesting
* Virtual buffer harvesting must never be restricted to the currently focused leaf node.
* When web content is focused, the platform tree harvester must ascend ancestor containers up to the enclosing `Document` / `UIA_DocumentControlTypeId` / WebArea root and harvest the virtual buffer downwards to ensure complete document context.

### Invariant 8: Zero Synthetic Latency
* The coordinator event loop, focus tracker, and tree providers must never introduce blocking delays or synthetic sleeps (e.g., `thread::sleep`).
* Responsiveness and low latency are foundational screen reader requirements.

---

## 3. Workspace Structure & Crate Ownership

```
crates/
├── bit_sr_core/               # Pure data models, Android-inspired tree, actions, roles, states
├── bit_sr_platform_windows/   # Win32 WH_KEYBOARD_LL, UIA CUIAutomation8, MSAA, Explorer heuristics
├── bit_sr_speech/             # Pluggable SpeechHub, SAPI 5 ISpVoice, MockSynthesizer
├── bit_sr_ui/                 # Slint accessible settings dashboard & extension marketplace
└── bit_sr_engine/             # Coordinator event loop, SpeechFormatter, FocusTracker, bit_sr CLI binary
```

### Dependency Flow
* `bit_sr_engine` depends on `bit_sr_core`, `bit_sr_speech`, and `bit_sr_platform_windows` (on Windows).
* `bit_sr_platform_windows` depends on `bit_sr_core`.
* `bit_sr_speech` depends on `bit_sr_core`.
* `bit_sr_core` has **no dependencies** on other workspace crates.

---

## 4. Coding Standards

1. **Rust Edition:** Edition 2024. Use modern idioms (pattern matching, RAII guards, explicit error handling).
2. **Zero Compiler Warnings:** All code must compile with `cargo check` and `cargo test` with **zero warnings** and zero errors.
3. **Explicit Error Handling:** Do not use `unwrap()` in production paths. Propagate errors via `Result<T, Error>` or log them cleanly.
4. **Unit Tests Required:** Every new feature, state transition, and formatting rule must have corresponding unit tests.
5. **Anti-Chatter Deduplication:** When handling focus and state changes, apply anti-chatter deduplication in [`FocusTracker`](crates/bit_sr_engine/src/tracker.rs) to avoid redundant speech.

---

## 5. Verification Workflow

Before committing any changes:

```powershell
# 1. Run all workspace unit tests (must pass with 0 failures)
cargo test --workspace

# 2. Check all targets, examples, and binaries compile with 0 warnings
cargo check --workspace --all-targets
```

> **Cloud-First Release Builds:** Do not run heavy release compilation (`cargo build --release`) on local contributor machines. Full binary optimization, packaging, and release publishing are handled entirely in the cloud by the GitHub Actions CI/CD pipeline on every push.

---

## 6. Git Commit Guidelines

Follow Conventional Commits:
* `feat(...)`: New features or drivers
* `fix(...)`: Bug fixes and edge case resolutions
* `perf(...)`: Latency and memory optimizations
* `docs(...)`: Documentation updates
* `refactor(...)`: Internal code improvements without behavioral changes
* `ci(...)`: CI/CD workflows and release automation

---

## 7. Upstream Contribution & Branching Conventions

1. **Target Upstream Directly:**
   - All Pull Requests must target the upstream repository (`ashu-choudhury/bit_sr`) with base branch `master`.
   - Do not submit pull requests targeting personal forks.
2. **Single-Branch Working Model (`main`):**
   - Perform all active development, bug fixes, and documentation directly on the local `main` branch.
   - Avoid creating multiple sprawling feature branches for small fixes. Commit clearly to `main`, push to `origin/main`, and open pull requests against `upstream/master`.
3. **Automated Bleeding-Edge Releases:**
   - Every push to the repository triggers the GitHub Actions CI/CD workflow (`.github/workflows/ci.yml`).
   - The workflow compiles the release binary on Windows x86_64, creates zipped standalone packages with SHA256 hashes, updates the `bleeding-edge` git tag, and publishes the pre-release automatically.
4. **Communication Language:**
   - All documentation, commit messages, PR descriptions, and discussions must strictly be in English (`en`).

