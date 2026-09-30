# bit_sr ⚡

[![Release](https://img.shields.io/badge/release-v0.1.0--alpha-blue.svg)](https://github.com/ashu-choudhury/bit_sr/releases)
[![Rust](https://img.shields.io/badge/rust-edition%202024-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green.svg)](#licensing--philosophy)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20(planned)-lightgrey.svg)](#architecture)

**`bit_sr`** is an experimental, next-generation, high-performance native desktop screen reader written from scratch in **Rust**. It features a platform-agnostic, Android-inspired accessibility tree hierarchy, sub-millisecond input hooks, a pluggable speech synthesis hub, and a sandboxed **WebAssembly (Wasm)** extension ecosystem.

---

## 🌟 Why `bit_sr`?

* **Bare-Metal Speed & Deterministic Latency:**  
  Traditional desktop screen readers written in Python or interpreted languages frequently encounter runtime stalls, garbage collection pauses, and Global Interpreter Lock (GIL) contention. `bit_sr` compiles directly to native machine code, dispatching operating system events to audio output with sub-millisecond latency.
* **Unified Mental & Programmatic Model:**  
  Inspired by Google Android's clean `AccessibilityNodeInfo` hierarchy, `bit_sr` normalizes complex operating system accessibility trees (Windows UI Automation, MSAA, and Linux AT-SPI2) into an elegant, relational node tree with standardized action execution (`Click`, `Expand`, `Scroll`, `Select`).
* **Instant Speech Interruption:**  
  Typing responsiveness is paramount. Every keypress immediately purges audio output queues asynchronously via hardware-accelerated drivers (`SPF_PURGEBEFORESPEAK`), eliminating speech lag while typing or browsing.
* **Safe, Sandboxed Extension Ecosystem:**  
  Say goodbye to arbitrary Python execution vulnerabilities. Community extensions, custom navigators, and synthesizer drivers will run inside a capability-secured WebAssembly (Wasm) sandbox.

---

## 🏛 Architecture Overview

```
                      ┌─────────────────────────────────────────┐
                      │             bit_sr (App)                │
                      └────────────────────┬────────────────────┘
                                           │
                               ┌───────────▼───────────┐
                               │     bit_sr_engine     │
                               │  (Coordinator & Loop) │
                               └─────┬───────────┬─────┘
                                     │           │
                 Events & Focus State│           │Spoken Text & Audio Purge
                                     │           │
         ┌───────────────────────────▼──┐     ┌──▼───────────────────────────┐
         │   bit_sr_platform_windows    │     │        bit_sr_speech         │
         │  (Low-Level Hooks & UIA COM) │     │    (SAPI 5 & Pluggable Hub)  │
         └──────────────┬───────────────┘     └──┬───────────────────────────┘
                        │                        │
                        └───────────┬────────────┘
                                    │
                         ┌──────────▼──────────┐
                         │     bit_sr_core     │
                         │  (Tree, Node, State)│
                         └─────────────────────┘
```

### Workspace Crates

| Crate | Path | Description |
| :--- | :--- | :--- |
| **`bit_sr_core`** | [`crates/bit_sr_core`](crates/bit_sr_core) | Platform-agnostic accessibility data structures, bidirectional tree hierarchy, Android-inspired action protocol, DFS reading order traversal, and spatial hit-testing. |
| **`bit_sr_platform_windows`** | [`crates/bit_sr_platform_windows`](crates/bit_sr_platform_windows) | Native Windows implementation: `WH_KEYBOARD_LL` low-level keyboard hook, COM MTA apartment management, Microsoft UI Automation (`CUIAutomation8`), MSAA WinEvent hooks, and specialized File Explorer (`CabinetWClass`) quirks handling. |
| **`bit_sr_speech`** | [`crates/bit_sr_speech`](crates/bit_sr_speech) | Pluggable text-to-speech hub and audio subsystem supporting Microsoft SAPI 5 (`ISpVoice`), instant speech interruption, voice selection, and mock drivers for unit testing. |
| **`bit_sr_engine`** | [`crates/bit_sr_engine`](crates/bit_sr_engine) | Central coordinator connecting platform events to speech, anti-chatter focus deduplication, human-readable speech formatting (*"Documents, list item, selected, 3 of 12"*), and global hotkey handling. |

---

## ⌨️ Live Prototype Hotkeys

When `bit_sr` is running, the following global hotkeys are active:

| Shortcut | Description |
| :--- | :--- |
| **Any Key** | Instantly interrupts / cuts off speech audio with zero latency |
| **`Insert + Tab`** or **`CapsLock + Tab`** | Repeat current focused element and full description |
| **`Insert + T`** or **`CapsLock + T`** | Announce active top-level window title |
| **`Insert + S`** or **`CapsLock + S`** | Toggle speech mode (**Talk** / **Mute**) |
| **`Insert + Ctrl + Up`** | Increase speech volume (+10%) |
| **`Insert + Ctrl + Down`** | Decrease speech volume (-10%) |
| **`Insert + Ctrl + Right`** | Increase speech rate (faster) |
| **`Insert + Ctrl + Left`** | Decrease speech rate (slower) |
| **`Ctrl + Alt + Q`** or **`Insert + Q`** | Cleanly terminate `bit_sr` |

---

## 🚀 Getting Started

### Prerequisites

* **Operating System:** Windows 10 or Windows 11 (x86_64)
* **Rust Toolchain:** Rust 1.85+ (Edition 2024 supported) with MSVC toolchain:
  ```powershell
  rustup default stable-x86_64-pc-windows-msvc
  ```

### Build & Run

1. **Clone the repository:**
   ```powershell
   git clone https://github.com/ashu-choudhury/bit_sr.git
   cd bit_sr
   git submodule update --init --recursive
   ```

2. **Run all tests (26 unit tests):**
   ```powershell
   cargo test --workspace
   ```

3. **Launch the live screen reader:**
   ```powershell
   cargo run --release --bin bit_sr
   ```

4. **Navigate:**
   - Switch to **Windows File Explorer** or the **Desktop** (`Win + D`).
   - Use the **Arrow keys**, **Tab**, and **Shift + Tab** to navigate.
   - SAPI 5 will announce files, folders, controls, and states in real time!
   - Press **`Ctrl + Alt + Q`** to exit cleanly.

---

## 📚 Technical Blueprints

The repository maintains comprehensive architectural documentation:

* **[Vision & Architecture (VISION.md)](VISION.md):** Architectural roadmap, threat modeling, and WebAssembly plugin engine design.
* **[Windows Subsystem Specification (WINDOWS.md)](WINDOWS.md):** Win32 syscall catalog, UIA COM interfaces, event coalescing, and all 9 File Explorer heuristics.
* **[Linux Subsystem Specification (LINUX.md)](LINUX.md):** AT-SPI2 D-Bus IPC protocol, `evdev` input handling, and Wayland desktop integration.
* **[Chromium & Web Specification (CHROMIUM.md)](CHROMIUM.md):** Chromium accessibility runtime, multi-process topology, `AXMode`, Browse Mode vs. Focus Mode state machine, quick nav, and all 12 Blink heuristics.
* **[Mozilla Firefox & Gecko Specification (FIREFOX.md)](FIREFOX.md):** Gecko accessibility architecture, "Cache the World" parent-process IPC, `IAccessible2`, `ISimpleDOMNode`, and Tor Browser anti-fingerprinting.
* **[Android Subsystem Specification (ANDROID.md)](ANDROID.md):** AOSP accessibility framework, `AccessibilityService`, `AccessibilityNodeInfo` mapping, touch exploration, decoupled dual-focus, and `accesskit_android`.
* **[Agent Guidelines (AGENTS.md)](AGENTS.md):** Coding invariants, COM safety rules, and architecture guardrails for contributors and AI agents.

---

## 📄 Licensing & Philosophy

`bit_sr` is distributed under the terms of both the **MIT License** and the **Apache License (Version 2.0)**.

* [MIT License](LICENSE-MIT)
* [Apache License, Version 2.0](LICENSE-APACHE)

### Why Dual MIT / Apache 2.0?

1. **Universal Ecosystem Integration:**  
   Permits individual crates (`bit_sr_core`, `bit_sr_speech`, `bit_sr_platform_windows`) to be embedded in operating system distributions, desktop environments, or game engines without copyleft friction.
2. **Patent Protection (Apache 2.0):**  
   Provides an explicit grant of patent rights from contributors to users, protecting assistive technology users from patent litigation.
3. **Open Freedom (MIT):**  
   Encourages worldwide research and community development in accessible computing.
