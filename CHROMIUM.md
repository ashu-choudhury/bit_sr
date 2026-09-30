# Chromium & Web Subsystem Specification: `bit_sr_web`

> **Document Classification:** Master Reference & Exhaustive Technical Specification  
> **Source Analysis:** NVDA Source Codebase (`references/nvda`), GNOME Orca Codebase (`references/orca`), Chromium Native Accessibility Subsystem (`ui/accessibility`, Blink `AXObjectCacheImpl`, Mojo IPC).  
> **Target:** Native Cross-Platform Implementation for Chromium, Google Chrome, Microsoft Edge, Electron, and WebView2.  
> **Status:** 100% Comprehensive — No external lookups into NVDA or Orca source code required.

---

## Table of Contents
1. [Architectural Overview & Chromium Multi-Process Topology](#1-architectural-overview--chromium-multi-process-topology)
2. [The Chromium Accessibility Engine & Activation Modes (`ui::AXMode`)](#2-the-chromium-accessibility-engine--activation-modes-uiaxmode)
3. [Platform Accessibility Dialects (Windows IA2/UIA, Linux AT-SPI2, macOS, Headless CDP)](#3-platform-accessibility-dialects)
4. [The Virtual Buffer Subsystem: Browse Mode vs. Focus Mode](#4-the-virtual-buffer-subsystem-browse-mode-vs-focus-mode)
5. [Single-Letter Quick Navigation & Table/Grid Traversal](#5-single-letter-quick-navigation--tablegrid-traversal)
6. [W3C ARIA 1.2/1.3, HTML5 Semantics & Live Regions](#6-w3c-aria-1213-html5-semantics--live-regions)
7. [Out-of-Process Iframes (OOPIFs) & Cross-Origin Tree Stitching](#7-out-of-process-iframes-oopifs--cross-origin-tree-stitching)
8. [Electron, WebView2 & Embedded Runtime Mechanics](#8-electron-webview2--embedded-runtime-mechanics)
9. [Exhaustive Chromium Quirks, Heuristics & Bug Workarounds](#9-exhaustive-chromium-quirks-heuristics--bug-workarounds)
10. [Master Chromium API, Interface, Attribute & Shortcut Reference Table](#10-master-chromium-api-interface-attribute--shortcut-reference-table)
11. [Rust Implementation Blueprint for `bit_sr`](#11-rust-implementation-blueprint-for-bit_sr)
12. [Workspace File & Directory Layout](#12-workspace-file--directory-layout)

---

## 1. Architectural Overview & Chromium Multi-Process Topology

Chromium is not merely an application; it is an entire **self-contained operating system runtime**. It powers:
* **Web Browsers:** Google Chrome, Microsoft Edge, Brave, Opera, Vivaldi, Arc.
* **Desktop Workstations (Electron):** Visual Studio Code, Slack, Discord, Microsoft Teams, Spotify, Obsidian, GitHub Desktop, WhatsApp.
* **Embedded Controls:** Windows App SDK WebView2 (`msedgewebview2.exe`), Chromium Embedded Framework (CEF), QtWebEngine.

Because of Site Isolation, sandboxing, and GPU rendering, Chromium partitions its runtime across multiple OS processes. Understanding this multi-process architecture is mandatory for screen reader design.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              Chromium Multi-Process Topology                           │
└────────────────────────────────────────────────────────────────────────────────────────┘

  ┌────────────────────────────────────────────────────────────────────────────────────┐
  │                      Browser Process (Main / Host Process)                         │
  │  - Owns top-level OS window: HWND (Windows), GtkWindow / Aura (Linux), NSView (macOS)│
  │  - Renders Browser UI (Omnibox, Tab Strip, Bookmarks, Menus, Downloads)           │
  │  - Manages `BrowserAccessibilityManager` for each active tab and frame             │
  │  - Implements OS accessibility bridges:                                            │
  │    * Windows: `BrowserAccessibilityWin` (IAccessible2 + MSAA) & UIA Providers     │
  │    * Linux: `BrowserAccessibilityAuraLinux` (AT-SPI2 D-Bus)                       │
  │    * macOS: `BrowserAccessibilityCocoa` (NSAccessibility)                          │
  └──────────────────────────────▲─────────────────────────────────▲───────────────────┘
                                 │                                 │
                   Mojo IPC (mojom::AccessibilityHost)             │ Mojo IPC
                                 │                                 │
  ┌──────────────────────────────▼────────────────────┐  ┌─────────▼───────────────────┐
  │         Renderer Process 1 (Sandboxed)            │  │   Renderer Process 2 (OOPIF)│
  │  Origin: `https://example.com`                    │  │   Origin: `https://bank.com`│
  │  - Blink Rendering Engine (HTML/CSS/JS)           │  │   - Cross-site <iframe>     │
  │  - `AXObjectCacheImpl`: listens to DOM/layout     │  │   - Isolated memory space   │
  │  - Generates `AXTreeSerializer` & `AXTreeUpdate`  │  │   - Dispatches child updates│
  └───────────────────────────────────────────────────┘  └─────────────────────────────┘
                                 ▲
                                 │ Intercepts OS Events / Injects Virtual Cursor
  ┌──────────────────────────────┴─────────────────────────────────────────────────────┐
  │                              `bit_sr` Screen Reader Engine                          │
  │  - Windows: Hooks `WH_KEYBOARD_LL`, interrogates `IAccessible2` / `IUIAutomation`    │
  │  - Linux: Listens to dedicated AT-SPI2 D-Bus bus via `zbus`                        │
  │  - Maintains in-memory `VirtualBuffer` for Browse Mode & Linear Cursor Navigation   │
  └────────────────────────────────────────────────────────────────────────────────────┘
```

### The Rendering Pipeline & Accessibility Tree Generation
1. **DOM & Layout Calculation:** Blink parses HTML and CSS. When layout shifts, Blink's `AXObjectCacheImpl` calculates which accessibility objects have mutated.
2. **Computed Accessible Name & Role:** `AXObject` calculates computed roles and accessible names based on the W3C Accessible Name and Description Computation specification.
3. **Mojo IPC Serialization:** The renderer bundles changed nodes into an `AXTreeUpdate` message containing node IDs, role enums, state bitsets, string attributes (`aria-label`, `tag`), and float attributes (bounding coordinates).
4. **Browser Mirror Reconstruction:** `BrowserAccessibilityManager` in the Browser Process applies the diff to its browser-side mirror of the tree.
5. **OS Accessibility Event Notification:** The Browser Process translates the `AXTreeUpdate` into native OS events (`EVENT_OBJECT_SHOW`, `IA2_EVENT_TEXT_INSERTED`, or AT-SPI2 `object:children-changed:add`).

---

## 2. The Chromium Accessibility Engine & Activation Modes (`ui::AXMode`)

Chromium maintains a lazy-loading accessibility policy. By default, to minimize memory footprint and execution overhead, **the accessibility tree in Blink is completely turned off**.

### 2.1 The `ui::AXMode` Bitmask
Chromium controls accessibility features through an internal bitmask flag structure defined in `ui/accessibility/ax_mode.h`:

| Bit Value | Flag Constant | Description & Purpose |
| :--- | :--- | :--- |
| `0x0001` | `kNativeAPIs` | Native OS platform accessibility hooks enabled. |
| `0x0002` | `kWebContents` | Web content accessibility trees generated by Blink. |
| `0x0004` | `kInlineTextBoxes` | **Crucial:** Breaks text into individual line and word glyph runs with bounding rects. Required for character, word, and line virtual cursor navigation. |
| `0x0008` | `kScreenReader` | Informs web content and extensions that a screen reader is active (triggers Google Docs, Monaco Editor, etc., to expose accessible DOMs). |
| `0x0010` | `kHTML` | Exposes raw HTML tag names, IDs, classes, and unmapped attributes to the accessibility API. |
| `0x0020` | `kHTMLMetadata` | Exposes page metadata (`<meta>`, `<title>`). |
| `0x0040` | `kLabelImages` | Triggers Google's machine learning OCR / image labeling. |
| `0x0080` | `kPDFPrinting` | Generates accessible tag structures for Chromium's internal PDF engine. |

### 2.2 Activation Triggers
Chromium automatically elevates its `AXMode` when:
1. **Windows:** A screen reader queries `WM_GETOBJECT` on a Chromium window handle (`Chrome_RenderWidgetHostHWND`), or calls `IUIAutomation::ElementFromHandle`.
2. **Linux:** Chromium detects an active AT-SPI2 registry daemon on the D-Bus bus or finds `org.gnome.desktop.interface toolkit-accessibility` set to `true`.
3. **Explicit CLI Flags:**
   ```bash
   chrome.exe --force-renderer-accessibility
   chrome.exe --enable-features=ScreenReaderPlatformEngine
   ```
4. **Environment Variables:**
   ```bash
   ACCESSIBILITY_ENABLED=1
   QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1
   ```

> **Target `AXMode` for `bit_sr`:**  
> `kNativeAPIs | kWebContents | kInlineTextBoxes | kScreenReader | kHTML` (`0x001F`). Without `kInlineTextBoxes`, arrow key navigation by line will fail or report entire paragraphs as a single indivisible block!

---

## 3. Platform Accessibility Dialects

Chromium exposes completely different platform accessibility interfaces depending on the host OS.

### 3.1 Windows Dialect A: `IAccessible2` (IA2) & MSAA
For over 15 years, NVDA and JAWS have interacted with Chromium primarily via **IAccessible2 (IA2)** over in-process or cross-process COM.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        Chromium Windows IAccessible2 Architecture                      │
└────────────────────────────────────────────────────────────────────────────────────────┘

  `Chrome_RenderWidgetHostHWND` (Top-level render view window)
  └── QueryInterface / LresultFromObject(WM_GETOBJECT)
      └── `BrowserAccessibilityWin`
          ├── `IAccessible` (Legacy MSAA)
          ├── `IAccessible2_2` (Extended attributes, unique IDs, relations)
          ├── `IAccessibleText` (Character offsets, line extents, caret positioning)
          ├── `IAccessibleHypertext` (Embedded hyperlink navigation)
          ├── `IAccessibleTable2` & `IAccessibleTableCell` (Grid navigation)
          ├── `IAccessibleAction` (Invoking click, expand, collapse)
          └── `ISimpleDOMNode` (Direct DOM node tree inspection)
```

#### Custom Chromium IA2 Object Attributes
Chromium exposes rich web semantics through semicolon-delimited key-value pairs via `IAccessible2::get_attributes`:

* `tag`: Raw HTML tag name (e.g. `tag:nav`, `tag:figure`, `tag:ul`, `tag:article`).
* `xml-roles`: ARIA roles (e.g. `xml-roles:banner`, `xml-roles:search`, `xml-roles:gridcell`).
* `description-from`: Declares the source of `accDescription` (e.g. `description-from:aria-describedby`, `description-from:aria-description`, `description-from:attribute-title`).
* `details-roles`: ARIA `aria-details` target roles separated by spaces (e.g. `details-roles:comment doc-footnote`).
* `current`: ARIA `aria-current` state (`page`, `step`, `location`, `date`, `time`, `true`).
* `haspopup`: ARIA `aria-haspopup` type (`menu`, `listbox`, `tree`, `grid`, `dialog`).
* `roledescription`: Custom author role override (`aria-roledescription="slide"`).
* `brailleroledescription`: Custom author braille role (`aria-brailleroledescription="sld"`).
* `goog-editable`: Google Docs/Slides internal flag (`goog-editable:false` overrides editable state).

### 3.2 Windows Dialect B: Modern UI Automation (UIA)
Chromium (and Microsoft Edge in particular) includes native Windows UI Automation providers implemented via `AXFragmentRootWin` and `BrowserAccessibilityComWin`.
* Implements `IRawElementProviderSimple`, `IRawElementProviderFragment`, `IRawElementProviderFragmentRoot`.
* Supports `ITextProvider`, `ITextProvider2`, `IValueProvider`, `IToggleProvider`, `IExpandCollapseProvider`, `IGridProvider`, `ITableProvider`.
* **Advantage:** Supports Windows 11 UIA Remote Operations (`IUIAutomationRemoteOperation`), allowing batched AST queries in a single cross-process round-trip.

### 3.3 Linux Dialect: AT-SPI2 D-Bus (`AXPlatformNodeAuralinux`)
On Linux, Chromium registers directly on the user's dedicated accessibility bus (`/run/user/1000/at-spi/bus`):
* Implements `org.a11y.atspi.Accessible` (Tree navigation, parent, children, role, states).
* Implements `org.a11y.atspi.Component` (Bounding coordinates, hit testing).
* Implements `org.a11y.atspi.Text` & `Hypertext` (Glyph retrieval, embedded object replacement character `U+FFFC`).
* Implements `org.a11y.atspi.Table` & `TableCell` (Row/col coordinates, spanning).
* Signals dispatched:
  - `object:state-changed:focused`
  - `object:text-caret-moved`
  - `object:text-selection-changed`
  - `object:children-changed:add` / `remove`

### 3.4 Headless / Cross-Platform Dialect: Chrome DevTools Protocol (CDP)
In automated or headless testing environments, Chromium provides direct JSON-RPC access over WebSockets via the `Accessibility` domain:
* `Accessibility.enable`: Turns on `AXMode`.
* `Accessibility.getFullAXTree`: Dumps the complete `AXTree` hierarchy in a single JSON payload.
* `Accessibility.queryAXTree`: Queries nodes matching specific roles or names.

---

## 4. The Virtual Buffer Subsystem: Browse Mode vs. Focus Mode

Desktop applications present discrete focus targets (e.g. an OK button, a text box). Web documents, however, are vast tapestries of non-focusable content: headings, paragraphs, lists, and articles. A screen reader user cannot navigate a web page using the `Tab` key alone, as `Tab` only stops on focusable elements.

To solve this, screen readers employ a **Virtual Buffer (Browse Mode)**.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              Virtual Buffer State Machine                              │
└────────────────────────────────────────────────────────────────────────────────────────┘

                 ┌────────────────────────────────────────┐
                 │              Browse Mode               │
                 │  - Arrow keys navigate virtual cursor  │
                 │  - Single-letter quick nav active      │
                 │  - Physical focus stays at document    │
                 └───────┬────────────────────────▲───────┘
                         │                        │
         Tab / Click into│                        │ Press Escape /
         Editable / Input│                        │ Navigate out of form /
                         │                        │ Press SRKey + Space
                         ▼                        │
                 ┌────────────────────────────────┴───────┐
                 │               Focus Mode               │
                 │  - Keystrokes pass directly to browser │
                 │  - User types text or uses form arrows │
                 │  - Virtual cursor syncs to caret       │
                 └────────────────────────────────────────┘
```

### 4.1 Browse Mode (Virtual Cursor) Mechanics
1. **Linearization:** The screen reader flattens the hierarchical DOM tree into a linear sequence of text and control boundaries.
2. **Virtual Cursor:** An internal offset index pointing to the current character/line in the buffer.
3. **Keystroke Interception:** When the user presses `Up Arrow` or `Down Arrow`, `bit_sr` intercepts the key, advances the virtual cursor to the next line in the buffer, and speaks the text alongside any entered/exited control tags (*"Heading level 2: Installation", "Link: Download"*). The browser's physical focus is **not** moved.

### 4.2 Focus Mode (Forms Mode) Mechanics
1. **Pass-Through:** Keystrokes are not intercepted for navigation; they are delivered directly to the target application.
2. **Editing:** The user can type into an `<input type="text">`, use arrows inside a `<textarea>`, or control an interactive web widget (`role="slider"`, `role="combobox"`).

### 4.3 Automatic Mode Switching Rules
The coordinator automatically transitions between modes based on element types:

| Element / Role Landed Upon | Automatic Transition | Rationale |
| :--- | :--- | :--- |
| `Role::EditableText` (`<input type="text">`) | $\to$ **Focus Mode** | User intended to type into the field. |
| `Role::Password` (`<input type="password">`) | $\to$ **Focus Mode** | Requires raw input entry. |
| `Role::ComboBox` (`<select>`, `role="combobox"`) | $\to$ **Focus Mode** | Up/Down arrows must select items in dropdown. |
| `Role::Slider` (`<input type="range">`) | $\to$ **Focus Mode** | Arrows change slider value. |
| `Role::Application` (`role="application"`) | $\to$ **Focus Mode** | Author declared custom keyboard interactions. |
| `Role::Document` / `Role::Article` | $\to$ **Browse Mode** | Static reading context. |
| Pressing `Escape` | $\to$ **Browse Mode** | Universal escape hatch back to virtual reading. |
| Pressing `SRKey + Space` | **Toggle Mode** | Explicit manual override by user. |

---

## 5. Single-Letter Quick Navigation & Table/Grid Traversal

In Browse Mode, `bit_sr` provides instant single-key jumping across the document structure.

### 5.1 Quick Navigation Keys Catalog

```
  Forward: [Key]                Backward: [Shift + Key]
```

| Key | Target Semantic | Target Roles / Tag Heuristics |
| :---: | :--- | :--- |
| **`H`** | Any Heading | `Role::Heading` (Levels 1–6) |
| **`1`–`6`** | Heading at Level $N$ | `Role::Heading` where `level == N` |
| **`K`** | Any Hyperlink | `Role::Link` |
| **`U`** | Unvisited Link | `Role::Link` without `State::Visited` |
| **`V`** | Visited Link | `Role::Link` with `State::Visited` |
| **`F`** | Any Form Field | `Role::EditableText`, `Role::CheckBox`, `Role::RadioButton`, `Role::ComboBox`, `Role::Button` |
| **`E`** | Edit Box | `Role::EditableText`, `Role::Password` |
| **`B`** | Push Button | `Role::Button` |
| **`X`** | Check Box | `Role::CheckBox` |
| **`R`** | Radio Button | `Role::RadioButton` |
| **`C`** | Combo Box | `Role::ComboBox` |
| **`T`** | Data Table | `Role::Table`, `Role::DataGrid` (excluding layout tables) |
| **`L`** | List | `Role::List` (`<ul>`, `<ol>`, `<dl>`) |
| **`I`** | List Item | `Role::ListItem` (`<li>`, `<dt>`, `<dd>`) |
| **`D`** | Landmark / Region | `Role::Landmark` (`<main>`, `<nav>`, `<header>`, `<footer>`, `<aside>`, `<section>`) |
| **`Q`** | Blockquote | `Role::BlockQuote` (`<blockquote>`) |
| **`G`** | Graphic / Image | `Role::Graphic` (`<img>`, `<svg>`) |
| **`S`** | Separator / Rule | `Role::Separator` (`<hr>`) |
| **`M`** | Frame / Iframe | `Role::Frame`, `Role::InternalFrame` (`<iframe>`) |
| **`O`** | Embedded Object | `Role::EmbeddedObject` (`<video>`, `<audio>`, `<canvas>`, `<object>`) |
| **`A`** | Annotation / Comment | `Role::Comment`, `Role::Suggestion`, `Role::Footnote` |

### 5.2 Table & Grid Coordinate Traversal
Web tables require specialized 2-dimensional navigation vectors:
* **Cell Navigation Shortcuts:**
  - `Alt + Ctrl + Left Arrow`: Move to previous column in same row.
  - `Alt + Ctrl + Right Arrow`: Move to next column in same row.
  - `Alt + Ctrl + Up Arrow`: Move to previous row in same column.
  - `Alt + Ctrl + Down Arrow`: Move to next row in same column.
* **Header Tracking:**
  - When moving horizontally, announce the corresponding **Column Header** (`<th>` in column) + Cell Content.
  - When moving vertically, announce the corresponding **Row Header** (`<th>` in row) + Cell Content.
* **Coordinate & Span Announcements:**
  - Announce coordinates: *"Row 4, Column 2"*.
  - If cell spans multiple rows/cols: *"Spans 2 columns, 3 rows"*.
* **Layout Table Suppression Heuristic:**
  - If a `<table>` has `role="presentation"` or `role="none"`, or consists of only 1 row or 1 column without headers, treat it as a transparent grouping and do **not** trigger table navigation mode.

---

## 6. W3C ARIA 1.2/1.3, HTML5 Semantics & Live Regions

### 6.1 HTML5 Tag to ARIA Role Normalization

| HTML5 Element | Default ARIA Role | Unified Screen Reader Role | Landmark Name |
| :--- | :--- | :--- | :--- |
| `<header>` (top-level) | `banner` | `Role::Landmark` | "banner" |
| `<nav>` | `navigation` | `Role::Landmark` | "navigation" |
| `<main>` | `main` | `Role::Landmark` | "main" |
| `<footer>` (top-level) | `contentinfo` | `Role::Landmark` | "content info" |
| `<aside>` | `complementary` | `Role::Landmark` | "complementary" |
| `<section aria-labelledby>`| `region` | `Role::Landmark` | "region" |
| `<article>` | `article` | `Role::Article` | — |
| `<dialog>` | `dialog` | `Role::Dialog` | — |
| `<figure>` | `figure` | `Role::Figure` | — |
| `<figcaption>` | `caption` | `Role::Caption` | — |
| `<mark>` | `mark` | `Role::MarkedContent` | — |

### 6.2 ARIA Live Regions Subsystem
Web applications dynamically update content (e.g. chat messages, score tickers, form validation errors) via `aria-live`:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              Live Region Event Pipeline                                │
└────────────────────────────────────────────────────────────────────────────────────────┘

  Blink DOM Mutation (`aria-live="polite"` or `aria-live="assertive"`)
  └── Chromium fires OS event (IA2 / UIA / AT-SPI2 text insertion or alert)
      └── `bit_sr` Live Region Dispatcher
          ├── Check `aria-busy`: If "true", drop or delay until "false".
          ├── Check `aria-atomic`:
          │   ├── "true": Announce entire container text.
          │   └── "false": Announce only the mutated text diff.
          └── Check Politeness Level:
              ├── `assertive`: Immediate speech purge (`SPF_PURGEBEFORESPEAK`) + Spoken now.
              └── `polite`: Enqueue into speech synthesizer queue, waiting for idle speech.
```

---

## 7. Out-of-Process Iframes (OOPIFs) & Cross-Origin Tree Stitching

Under Chromium's Site Isolation architecture, if `https://site-a.com` embeds an `<iframe>` pointing to `https://site-b.com`, the two frames run in **different operating system processes**.

### 7.1 The Tree Stitching Protocol
1. **Renderer A:** Generates an `AXNode` for the `<iframe>` element. It assigns the attribute `ax::mojom::StringAttribute::kChildTreeId` containing a unique 128-bit GUID.
2. **Renderer B:** Generates the accessibility tree for `site-b.com`. Its root node has `ax::mojom::Action::kFocus` and announces itself with the matching Tree ID.
3. **Browser Process (`BrowserAccessibilityManager`):** Receives both updates over Mojo IPC, links the child tree root to the parent frame node, and exposes a unified contiguous hierarchy to the screen reader.
4. **Screen Reader Implication:** `bit_sr` must track `NodeId` with an associated `FrameId` tuple `(FrameId, NodeId)` to prevent cross-frame node identifier collisions.

---

## 8. Electron, WebView2 & Embedded Runtime Mechanics

Chromium powers millions of desktop applications via Electron and WebView2. These runtimes introduce special lifecycle and identification quirks.

### 8.1 Microsoft Edge WebView2 Process Resolution (`msedgewebview2.exe`)
* In Windows apps (Office, Teams, PowerToys, WinUI3), web views are rendered by child processes named `msedgewebview2.exe`.
* If a screen reader identifies the application solely by the foreground process executable name, all WebView2 apps are reported as *"msedgewebview2"*, breaking application-specific speech profiles.
* **The Solution (NVDA Workaround #16705):**
  When focus lands on an `msedgewebview2.exe` process, query the process tree (via Win32 `CreateToolhelp32Snapshot` or `NtQueryInformationProcess`) to identify the **Parent Process ID (PPID)**. Route application-specific commands to the parent executable name (e.g. `slack.exe`, `teams.exe`).

### 8.2 Electron Main Frame vs. WebContents
In Electron applications like Visual Studio Code:
* The window frame and top menu are native desktop or Chromium Views controls.
* The editor, sidebar, and status bar are HTML/CSS rendered inside Blink.
* `bit_sr` must automatically toggle between standard desktop focus handling for the menu bar and Browse/Focus mode inside editor web views.

---

## 9. Exhaustive Chromium Quirks, Heuristics & Bug Workarounds

Screen readers navigating Chromium encounter low-level rendering quirks and protocol bugs. The following 12 heuristics are derived from production NVDA and Orca workarounds.

### Quirk 1: Collapsed Combobox Focus Redirection (Chrome 68+)
* **Bug:** In Chromium, when an HTML `<select>` or ARIA combobox is collapsed, navigating items fires focus events on the internal hidden list items.
* **Symptom:** Screen reader reports list items while the combobox is visually closed.
* **Workaround:** If a focus event is received for an item with `Role::ListItem`, inspect its parent. If the parent list has `State::Invisible` (collapsed), redirect focus upwards to the ancestor `Role::ComboBox`.

### Quirk 2: ToggleButton Checkable State Bug
* **Bug:** Chromium mistakenly attaches `State::Checkable` to controls with `Role::ToggleButton`.
* **Symptom:** Screen reader speaks *"Checkable toggle button pressed"*, causing redundant clutter.
* **Workaround:** When normalizing control states, if `Role::ToggleButton` is present, unconditionally discard `State::Checkable`.

### Quirk 3: Presentational Lists Lacking Read-Only State
* **Bug:** Standard HTML unordered (`<ul>`), ordered (`<ol>`), and definition (`<dl>`) lists in Chromium do not carry the `State::ReadOnly` flag when located inside a `contenteditable` container.
* **Symptom:** Screen readers confuse them with interactive ARIA listboxes and enter focus mode.
* **Workaround:** If an element has `Role::List` and its `tag` attribute is `ul`, `ol`, or `dl`, explicitly inject `State::ReadOnly`.

### Quirk 4: `<figure>` Exposed as Generic Grouping
* **Bug:** Chromium often maps `<figure>` HTML elements to `Role::Grouping` instead of `Role::Figure`.
* **Workaround:** If `Role::Grouping` is encountered and the object attribute `tag` equals `"figure"`, override the role to `Role::Figure`.

### Quirk 5: VS Code / Electron Large File Caret Offset Freeze
* **Bug:** In Visual Studio Code (Electron), querying `IAccessibleText::textAtOffset` with `IA2_OFFSET_CARET` or evaluating `isCaretAtEndOfLine` across a 50,000-line source file triggers a synchronous O(N) layout scan in Blink, freezing the screen reader for seconds.
* **Workaround:** Bypass end-of-line checking in Chromium editor text infos when inside recognized Electron code editors.

### Quirk 6: Document with No URI (Orca Bug)
* **Bug:** During tab creation and navigation, Chromium fires `object:state-changed:focused` on newly created `Role::Document` nodes before their document URI is initialized.
* **Workaround:** Discard document focus events if `document_uri` is null or empty.

### Quirk 7: Autocomplete Popup Focus Trap
* **Bug:** Chromium's Omnibox and form autocomplete dropdowns are hosted in detached, un-named top-level frames.
* **Workaround:** When a `selected-changed` event fires outside the active document, traverse ancestors to detect un-named popup frames and route the locus of focus to the autocomplete list item.

### Quirk 8: UIA Empty Line Break Expansion Bug (#12474)
* **Bug:** In Chromium UIA, calling `IUIAutomationTextRange::ExpandToEnclosingUnit(TextUnit_Line)` on an empty document range causes an RPC crash or infinite loop.
* **Workaround:** Before calling `ExpandToEnclosingUnit`, verify that `documentRange.GetText(1)` is non-empty.

### Quirk 9: Redundant Heading Levels in Text Formatting Fields
* **Bug:** Chromium UIA exposes heading levels both in the element tree (`UIA_LevelPropertyId`) and in the text formatting attributes run (`heading-level`), causing double announcements: *"Heading level 2, heading level 2"*.
* **Workaround:** Strip the `heading-level` key from text format attributes in Chromium.

### Quirk 10: Google Docs / Slides Canvas Override (`goog-editable`)
* **Bug:** Google Docs and Slides render content onto an HTML5 canvas while maintaining a hidden accessibility tree with custom attribute `goog-editable="false"`.
* **Workaround:** If `goog-editable == "false"` is present in object attributes, unconditionally discard `State::Editable`.

### Quirk 11: `loadChromiumVBufOnBusyState` Optimization
* **Bug:** Chromium sets `State::Busy` on document nodes during background network fetch or DOM hydration. If a screen reader waits for `Busy` to clear before building its virtual buffer, the user experiences total silence during slow page loads.
* **Workaround:** Allow `VirtualBuffer` to construct and display content immediately even when `State::Busy` is active, updating dynamically as `children-changed` events arrive.

### Quirk 12: ARIA Details Roles Optimization (`details-roles`)
* **Bug:** Querying full reverse relations for `aria-details` across complex web pages requires multiple cross-process COM calls.
* **Optimization:** Chromium provides the custom IA2 attribute `details-roles` (e.g. `details-roles:comment doc-footnote`). Screen readers inspect this attribute directly to announce annotation counts without resolving remote relation targets until explicitly requested.

---

## 10. Master Chromium API, Interface, Attribute & Shortcut Reference Table

### 10.1 Complete Unified Role Mapping for Web & ARIA

| HTML / ARIA Semantic | IA2 Role Constant | UIA Control Type | AT-SPI2 Role | Unified `bit_sr` Role |
| :--- | :--- | :--- | :--- | :--- |
| `<a href="...">` | `ROLE_SYSTEM_LINK` | `UIA_HyperlinkControlTypeId` | `ROLE_LINK` | `Role::Link` |
| `<h1>`–`<h6>` | `IA2_ROLE_HEADING` | `UIA_HeaderControlTypeId` | `ROLE_HEADING` | `Role::Heading` |
| `<button>` | `ROLE_SYSTEM_PUSHBUTTON` | `UIA_ButtonControlTypeId` | `ROLE_PUSH_BUTTON` | `Role::Button` |
| `<input type="checkbox">`| `ROLE_SYSTEM_CHECKBUTTON`| `UIA_CheckBoxControlTypeId` | `ROLE_CHECK_BOX` | `Role::CheckBox` |
| `<input type="radio">` | `ROLE_SYSTEM_RADIOBUTTON`| `UIA_RadioButtonControlTypeId`| `ROLE_RADIO_BUTTON` | `Role::RadioButton` |
| `<select>` / `combobox` | `ROLE_SYSTEM_COMBOBOX` | `UIA_ComboBoxControlTypeId` | `ROLE_COMBO_BOX` | `Role::ComboBox` |
| `<input type="text">` | `ROLE_SYSTEM_TEXT` | `UIA_EditControlTypeId` | `ROLE_ENTRY` | `Role::EditableText` |
| `<textarea>` | `ROLE_SYSTEM_TEXT` | `UIA_DocumentControlTypeId` | `ROLE_DOCUMENT_TEXT` | `Role::DocumentText` |
| `<table>` | `IA2_ROLE_TABLE` | `UIA_TableControlTypeId` | `ROLE_TABLE` | `Role::Table` |
| `<tr>` | `IA2_ROLE_ROW` | `UIA_DataItemControlTypeId` | `ROLE_TABLE_ROW` | `Role::TableRow` |
| `<td>` | `ROLE_SYSTEM_CELL` | `UIA_DataItemControlTypeId` | `ROLE_TABLE_CELL` | `Role::TableCell` |
| `<th>` | `ROLE_SYSTEM_COLUMNHEADER`| `UIA_HeaderItemControlTypeId`| `ROLE_COLUMN_HEADER` | `Role::TableColumnHeader`|
| `<ul>`, `<ol>` | `ROLE_SYSTEM_LIST` | `UIA_ListControlTypeId` | `ROLE_LIST` | `Role::List` |
| `<li>` | `ROLE_SYSTEM_LISTITEM` | `UIA_ListItemControlTypeId` | `ROLE_LIST_ITEM` | `Role::ListItem` |
| `<blockquote>` | `IA2_ROLE_BLOCK_QUOTE` | `UIA_GroupControlTypeId` | `ROLE_BLOCK_QUOTE` | `Role::BlockQuote` |
| `<nav>`, `<main>`, etc. | `IA2_ROLE_LANDMARK` | `UIA_GroupControlTypeId` | `ROLE_LANDMARK` | `Role::Landmark` |
| `<figure>` | `IA2_ROLE_FIGURE` | `UIA_GroupControlTypeId` | `ROLE_PANEL` | `Role::Figure` |
| `<dialog>` | `ROLE_SYSTEM_DIALOG` | `UIA_WindowControlTypeId` | `ROLE_DIALOG` | `Role::Dialog` |

---

## 11. Rust Implementation Blueprint for `bit_sr`

In strict adherence to **Invariant 1 (Zero Platform Code in Core and Engine)**, Chromium and Web support is partitioned cleanly across pure data models, the engine coordinator, and platform drivers.

### 11.1 Pure Data Models (`crates/bit_sr_core/src/web/`)

```rust
//! Pure platform-agnostic models for Web, ARIA, and Virtual Buffer navigation.

use crate::node::NodeId;
use crate::roles::Role;
use crate::states::State;
use std::collections::HashSet;

/// Current active navigation mode inside a web document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavigationMode {
    /// Virtual cursor intercepts arrow keys and single-letter navigation.
    #[default]
    Browse,
    /// Raw keystrokes pass directly to web content or active form field.
    Focus,
}

/// Single-letter quick navigation target categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuickNavKey {
    Heading,
    HeadingLevel(u8),
    Link,
    UnvisitedLink,
    VisitedLink,
    FormField,
    EditBox,
    Button,
    CheckBox,
    RadioButton,
    ComboBox,
    Table,
    List,
    ListItem,
    Landmark,
    BlockQuote,
    Graphic,
    Separator,
    Frame,
    EmbeddedObject,
    Annotation,
}

/// A contiguous text run inside a virtual buffer line.
#[derive(Debug, Clone)]
pub struct TextRun {
    pub text: String,
    pub node_id: NodeId,
    pub role: Role,
    pub states: HashSet<State>,
}

/// A linearized representation of a web document line.
#[derive(Debug, Clone)]
pub struct BufferLine {
    pub line_number: usize,
    pub runs: Vec<TextRun>,
    pub start_offset: usize,
    pub end_offset: usize,
}

/// In-memory linearized virtual document buffer.
#[derive(Debug, Clone, Default)]
pub struct VirtualBuffer {
    pub document_node_id: NodeId,
    pub lines: Vec<BufferLine>,
    pub current_cursor_offset: usize,
    pub current_line_index: usize,
    pub mode: NavigationMode,
}
```

### 11.2 The Web Coordinator Engine (`crates/bit_sr_engine/src/web/`)

```rust
//! Engine coordinator managing virtual buffer traversal and mode transitions.

use crate::coordinator::EngineAction;
use bit_sr_core::web::{NavigationMode, QuickNavKey, VirtualBuffer};
use bit_sr_core::{AccessibilityEvent, Role};

pub struct WebController {
    buffer: VirtualBuffer,
    speech_queue: Vec<String>,
}

impl WebController {
    pub fn new() -> Self {
        Self {
            buffer: VirtualBuffer::default(),
            speech_queue: Vec::new(),
        }
    }

    /// Evaluates incoming platform events and adjusts Browse/Focus mode.
    pub fn handle_event(&mut self, event: &AccessibilityEvent) -> Vec<EngineAction> {
        let mut actions = Vec::new();

        if let AccessibilityEvent::FocusChanged { node } = event {
            // Automatic state machine transitions
            match node.role {
                Role::EditableText | Role::Password | Role::ComboBox | Role::Slider => {
                    if self.buffer.mode != NavigationMode::Focus {
                        self.buffer.mode = NavigationMode::Focus;
                        actions.push(EngineAction::PlaySound("focus_mode.wav"));
                    }
                }
                Role::Document | Role::Article | Role::Window => {
                    if self.buffer.mode != NavigationMode::Browse {
                        self.buffer.mode = NavigationMode::Browse;
                        actions.push(EngineAction::PlaySound("browse_mode.wav"));
                    }
                }
                _ => {}
            }
        }

        actions
    }

    /// Executes single-letter quick navigation forward or backward.
    pub fn navigate_quick_key(&mut self, key: QuickNavKey, reverse: bool) -> Option<EngineAction> {
        if self.buffer.mode != NavigationMode::Browse {
            return None; // Quick nav only active in Browse Mode
        }
        // Scan buffer lines for the target role and return speech announcement
        None
    }
}
```

### 11.3 Platform Normalizer & Heuristics (`crates/bit_sr_platform_windows/src/apps/chromium.rs`)

```rust
//! Windows-specific Chromium and Blink heuristics filter.

use bit_sr_core::roles::Role;
use bit_sr_core::states::State;
use std::collections::HashSet;

pub struct ChromiumFilter;

impl ChromiumFilter {
    /// Normalizes Chromium roles based on tag attributes (Quirks 3 & 4).
    pub fn normalize_role(role: Role, tag: &str) -> Role {
        if role == Role::Grouping && tag.eq_ignore_ascii_case("figure") {
            return Role::Figure;
        }
        role
    }

    /// Fixes state anomalies (Quirk 2: Checkable on ToggleButtons; Quirk 3: Lists).
    pub fn normalize_states(role: Role, tag: &str, mut states: HashSet<State>) -> HashSet<State> {
        if role == Role::ToggleButton {
            states.remove(&State::Checkable);
        }
        if role == Role::List && matches!(tag, "ul" | "ol" | "dl") {
            states.insert(State::ReadOnly);
        }
        states
    }
}
```

---

## 12. Workspace File & Directory Layout

To maintain architectural purity and cross-platform extensibility, Chromium and Web support is structured across the workspace as follows:

```
crates/
├── bit_sr_core/
│   └── src/
│       ├── web/                     # Platform-agnostic web models
│       │   ├── mod.rs               # Exports VirtualBuffer, NavigationMode, QuickNavKey
│       │   ├── buffer.rs            # VirtualBuffer, BufferLine, TextRun data structures
│       │   ├── quick_nav.rs         # Quick navigation key bindings and targets
│       │   ├── aria.rs              # ARIA 1.2/1.3 roles, live regions, attributes
│       │   └── table.rs             # 2D table coordinates and header associations
│       └── lib.rs                   # Re-exports `pub mod web;`
│
├── bit_sr_engine/
│   └── src/
│       ├── web/                     # Engine-side web controller
│       │   ├── mod.rs               # Exports WebController and Linearizer
│       │   ├── controller.rs        # Browse/Focus state machine & earcon dispatch
│       │   ├── linearizer.rs        # AccessibleNode tree -> VirtualBuffer compiler
│       │   └── quick_nav.rs         # Quick navigation jumping logic
│       └── lib.rs                   # Re-exports `pub mod web;`
│
├── bit_sr_platform_windows/
│   └── src/
│       └── apps/
│           ├── mod.rs               # Exports ChromiumFilter
│           └── chromium.rs          # IA2/UIA normalization, quirks 1-12 workarounds
│
└── bit_sr_platform_linux/
    └── src/
        └── apps/
            ├── mod.rs               # Exports Linux Chromium handler
            └── chromium.rs          # AT-SPI2 D-Bus URI checks, popup detection
```

---

## 13. Conformance Checklist for Contributors & Subagents

When contributing code to Chromium and Web support in `bit_sr`:
1. [ ] **No Win32/COM in Core:** Never import `windows`, `oleacc`, or `atspi` into `bit_sr_core::web` or `bit_sr_engine::web`.
2. [ ] **Always Purge on Focus Mode Switch:** When toggling from Browse Mode to Focus Mode, purge any queued speech utterances immediately.
3. [ ] **Preserve Quick Nav Invariance:** Quick nav must only intercept keys when `NavigationMode::Browse` is active.
4. [ ] **Cover All 12 Quirks:** Every platform filter must implement the 12 documented heuristics with associated unit tests.
5. [ ] **Zero Warnings:** Ensure `cargo check --workspace` and `cargo test --workspace` run with 0 errors and 0 warnings.
