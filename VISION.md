# bit_sr — Project Vision & Architecture Specification

## 1. Executive Summary

**`bit_sr`** is an open-source, next-generation, native desktop screen reader engineered in **Rust**. It is designed from the ground up for speed, rock-solid stability, cross-platform portability, and uncompromising security.

Unlike incumbent screen readers that rely on Python runtimes with Global Interpreter Locks (GIL) and uncontained extension systems, `bit_sr` delivers:
- **Sub-millisecond event processing latency** via native compiled code.
- **A unified cross-platform accessibility tree** abstracting Windows (UI Automation / MSAA) and Linux (AT-SPI2).
- **A sandboxed WebAssembly (Wasm) plugin ecosystem** that eliminates arbitrary code execution risks and establishes capability-based security for community extensions.

---

## 2. The Problem Landscape & Motivation

### The Existing Ecosystem
| Screen Reader | Platform | Language | Strengths | Critical Weaknesses |
| :--- | :--- | :--- | :--- | :--- |
| **JAWS** | Windows | C++ / Proprietary Script | Enterprise standard, legacy app support | Bloated, expensive ($1,000+), closed-source, proprietary |
| **NVDA** | Windows | Python / C++ | Open source, huge community, rich feature set | Python GIL causes speech/input lag under heavy load; **Zero plugin sandboxing** |
| **Orca** | Linux | Python / C | Default Linux screen reader | Python overhead, sluggish tree walking, struggles with Wayland ecosystem changes |

### Core Problems `bit_sr` Solves

1. **The Python GIL & Latency Problem:**
   Screen readers operate in real-time under intense constraints. During rapid typing or navigation of complex web applications (e.g., Google Docs, VS Code), hundreds of accessibility events fire per second. In Python-based screen readers, garbage collection pauses and GIL contention introduce audio stutter and input latency. Rust provides zero-cost abstractions, deterministic performance, and fearless concurrency.

2. **The Add-on Security Crisis:**
   In NVDA, add-ons have full access to the user's operating system with arbitrary Python execution privileges. A malicious or compromised add-on can install keyloggers, read passwords, access the network, or encrypt user data. In Windows Secure Desktop mode (e.g., UAC prompts), this can compromise the entire OS.
   **`bit_sr` solves this by isolating every add-on inside a WebAssembly sandbox.**

3. **Desktop Fragmentation:**
   Assistive technology is heavily fragmented across operating systems. Blind developers and users who switch between Windows and Linux are forced to relearn completely different keyboard shortcuts, speech formatting conventions, and extension ecosystems. `bit_sr` bridges this gap with a single unified mental model.

---

## 3. Core Architectural Pillars

```
                                  ┌───────────────────────────────┐
                                  │          bit_sr App           │
                                  │   (Orchestrator & Main Loop)  │
                                  └──────────────┬────────────────┘
                                                 │
                   ┌─────────────────────────────┼─────────────────────────────┐
                   │                             │                             │
    ┌──────────────▼──────────────┐┌─────────────▼─────────────┐┌──────────────▼──────────────┐
    │       bit_sr_platform       ││       bit_sr_speech       ││       bit_sr_plugin         │
    │  (Unified Accessibility &   ││ (Pluggable Speech Engine: ││ (Sandboxed Wasmtime Runtime │
    │       Input Hooks)          ││ OneCore, SAPI, Piper, etc)││  with Capability Security)  │
    └──────────────┬──────────────┘└───────────────────────────┘└──────────────┬──────────────┘
                   │                                                           │
        ┌──────────┴──────────┐                                     ┌──────────┴──────────┐
        │                     │                                     │  Sandboxed Plugins  │
 ┌──────▼──────┐       ┌──────▼──────┐                              │  - OCR Tools        │
 │   Windows   │       │    Linux    │                              │  - App Scripts      │
 │ UIAutomation│       │   AT-SPI2   │                              │  - Custom Hotkeys   │
 │   / MSAA    │       │  via zbus   │                              │  - Speech Filters   │
 └─────────────┘       └─────────────┘                              └─────────────────────┘
```

### Pillar 1: Unified Accessibility Tree & Virtual Buffer (`bit_sr_core`)
- Models UI controls into normalized structures: `Role` (Button, Window, EditField, Heading, Link, Table, etc.), `State` (Focused, Selected, Expanded, Checked), and `BoundingRect`.
- Provides a high-performance **Virtual Buffer** engine for web documents (Chromium, Firefox) to enable seamless linear reading (browse mode) with quick navigation keys (H for headings, K for links, T for tables, etc.).

### Pillar 2: Platform Abstraction Layer (`bit_sr_platform`)
- **Windows (`bit_sr_platform_windows`):**
  - Interfaces directly with Microsoft UI Automation (`IUIAutomation`) and WinEvents/MSAA via the official `windows` crate.
  - Registers low-level keyboard hooks (`SetWindowsHookExW`) to intercept hotkeys with zero perceptible latency.
- **Linux (`bit_sr_platform_linux`):**
  - Interfaces with the AT-SPI2 D-Bus protocol using pure Rust (`atspi` and `zbus` crates).
  - Handles input capture across modern Wayland compositors (via standard protocols) and X11/evdev.

### Pillar 3: Ultra-Low-Latency Speech Subsystem (`bit_sr_speech`)
- Abstracted speaker interface supporting prioritized speech queues, instant speech cancellation on keypress (speech interruption), and speech rate/pitch pitch adjustments.
- Backends supported:
  - Windows: Windows OneCore Speech API & SAPI 5.
  - Linux: Speech Dispatcher (`speechd`).
  - Cross-platform / Neural: Offline neural synthesizers like **Piper TTS** (ONNX-based) and **eSpeak NG**.

### Pillar 4: Sandboxed Wasm Extension Runtime (`bit_sr_plugin`)
- Extension execution is powered by **Wasmtime** or **Wasmi** utilizing the WebAssembly Component Model.
- **Capability-Based Permissions:** Every add-on must provide a manifest specifying its required capabilities:
  - `speech:filter`: Read and transform outgoing speech text.
  - `hotkey:register`: Register new shortcuts.
  - `accessibility:inspect`: Read properties of the focused element.
  - `network:fetch`: Explicitly scoped HTTP requests (e.g., for AI image description).
  - ❌ **No arbitrary filesystem execution or shell process spawning.**

### Pillar 5: Accessible Native GUI & Extension Storefront (`bit_sr_ui`)
- Powered by **Slint** (with native AccessKit integration).
- **Retained-mode accessibility:** Empirically verified to provide 100% accessible controls (combo boxes, sliders, checkboxes, text fields) to screen readers out of the box.
- Compiles natively to a single binary with zero external DLLs, avoiding WebView/Electron memory overhead and multi-platform UI code fragmentation.

---

## 4. Reference Codebases

To accelerate engineering and accurately handle platform-specific edge cases, the repository includes two primary reference codebases as submodules under `references/`:

1. **`references/nvda`** ([nvaccess/nvda](https://github.com/nvaccess/nvda)):
   - Serves as the primary reference for Windows UI Automation event handling, MSAA/IA2 heuristics, text range caret tracking, and virtual buffer layout algorithms.
2. **`references/orca`** ([GNOME/orca](https://github.com/GNOME/orca)):
   - Serves as the primary reference for Linux AT-SPI2 event consumption, structural navigation, and braille/speech generation rules.

---

## 5. Phased Roadmap

*(For the detailed, actionable 5-step implementation plan, see [`ROADMAP.md`](ROADMAP.md).)*

### Phase 1: Windows MVP & Core Loop
- [x] Initialize Cargo workspace (`bit_sr_core`, `bit_sr_platform_windows`, `bit_sr_speech`, `bit_sr_engine`).
- [x] Implement asynchronous speech output with instant interruption support (Windows SAPI 5 dedicated worker).
- [x] Connect Windows UI Automation Focus Changed event listener with focus event coalescing.
- [x] Speak element name, role, and states upon focus change (Tab / Alt+Tab) with localized multi-language catalogs.

### Phase 2: Input Handling, Text Editing & Review Navigation
- [x] Implement low-level keyboard hook (`WH_KEYBOARD_LL`) with sub-50ns latency and dedicated decoding worker.
- [x] Implement dynamic SR modifier tracker (CapsLock / Insert) and double-tap hardware CapsLock toggle.
- [x] Implement text caret reading (Character, Word, Line, Selection, Home/End, Blank detection, typing echo).
- [ ] Implement object navigation / review cursor (parent, child, previous sibling, next sibling).
- [ ] Implement Windows system tray daemon lifecycle & single-instance lock.
- [ ] Implement native Settings GUI dashboard using Slint (`bit_sr_ui`).

### Phase 3: WebAssembly Plugin Engine & Storefront
- [ ] Implement Wasm plugin host using `wasmtime`.
- [ ] Define WebAssembly host functions (speech event interceptor, focus event hook, custom synthesizer driver).
- [ ] Build Slint-based Add-on Storefront / Marketplace tab.

### Phase 4: Virtual Buffer & Advanced Web Navigation
- [ ] Flatten HTML DOM trees from Chromium and Firefox into navigable linear text buffers.
- [ ] Implement quick navigation keys (H, K, F, T, B, etc.).
- [ ] Browse Mode vs. Focus Mode automatic switching.

### Phase 5: Linux Desktop Integration
- [ ] Implement `bit_sr_platform_linux` connecting to AT-SPI2 via `atspi` / `zbus`.
- [ ] Map AT-SPI2 events to `bit_sr_core` events.
- [ ] Connect Linux input capture and Speech Dispatcher (`speechd`).

### Phase 6: Braille Display Hardware Integration
- [ ] Driver abstraction for refreshable braille displays via USB / Bluetooth.
- [ ] Liblouis translation integration for contracted and uncontracted Braille.
