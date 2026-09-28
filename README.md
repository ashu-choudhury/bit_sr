# bit_sr

**`bit_sr`** is an experimental next-generation, high-performance desktop screen reader written natively in **Rust**, featuring a **WebAssembly (Wasm) sandboxed extension ecosystem** and cross-platform desktop support for **Windows** and **Linux**.

---

## Key Pillars

1. **Native Speed & Sub-Millisecond Latency:**  
   Unlike Python-based screen readers burdened by Global Interpreter Lock (GIL) stalls and garbage collection pauses, `bit_sr` delivers deterministic, low-latency audio feedback and keystroke handling.
2. **Unified Accessibility Tree:**  
   Normalizes Microsoft UI Automation (`IUIAutomation`) / MSAA on Windows and AT-SPI2 D-Bus on Linux into a unified mental and programmatic model.
3. **Sandboxed WebAssembly Plugins:**  
   Eliminates arbitrary code execution vulnerabilities by isolating third-party add-ons inside a strict WebAssembly sandbox with explicit capability-based security.
4. **Desktop Focused:**  
   Engineered specifically for full desktop fidelity, starting with deep integration for Windows File Explorer (`CabinetWClass`) and Linux GNOME Files (`Nautilus`).

---

## Technical Specifications

The repository includes complete, self-contained architectural blueprints and system call specifications:

- **[Project Vision & Architecture Specification (VISION.md)](VISION.md):**  
  High-level vision, problem landscape, multi-phase roadmap, and Wasm capability model.
- **[Windows Subsystem & API Integration Specification (WINDOWS.md)](WINDOWS.md):**  
  Exhaustive catalog of Win32 hooks (`WH_KEYBOARD_LL`), UI Automation COM interfaces, MSAA fallback, out-of-process memory reading for common controls, and all 9 File Explorer quirks.
- **[Linux Subsystem & AT-SPI2 Integration Specification (LINUX.md)](LINUX.md):**  
  Exhaustive catalog of AT-SPI2 D-Bus protocols, `org.a11y.atspi.Cache` subtree batching, kernel `evdev`/Wayland input capture, and GNOME Shell/Nautilus navigation rules.

---

## Licensing & Philosophy

`bit_sr` is distributed under the terms of both the **MIT License** and the **Apache License (Version 2.0)**.

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

### Why Dual MIT / Apache 2.0?

In assistive technology, many existing screen readers use GPL licenses. For `bit_sr`, we deliberately chose the **Dual MIT / Apache 2.0** model (the standard licensing convention of the Rust language and ecosystem) for the following reasons:

1. **Universal Ecosystem Integration:**  
   Permits individual crates (such as `bit_sr_platform_windows`, `bit_sr_platform_linux`, or `bit_sr_speech`) to be embedded, linked, or adopted by operating system distributions, desktop environments, desktop toolkits, or game engines without legal or copyleft friction.
2. **Patent Protection (Apache 2.0):**  
   The Apache 2.0 license provides an explicit grant of patent rights from contributors to users, protecting the community from patent ambush.
3. **Simplicity & Freedom (MIT):**  
   The MIT license allows maximum permissive freedom for developers and researchers building assistive tools.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
