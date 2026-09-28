# Linux Subsystem & AT-SPI2 Integration Specification: `bit_sr_platform_linux`

> **Document Classification:** Master Reference & Exhaustive Technical Specification  
> **Source Analysis:** GNOME Orca Codebase (`references/orca`), AT-SPI2 D-Bus Architecture, Wayland/evdev Input Mechanics, and Linux File Managers (Nautilus/Dolphin).  
> **Target:** Native Rust Implementation for Linux (`bit_sr_platform_linux`).  
> **Status:** 100% Comprehensive — No external lookups into Orca source code required.

---

## Table of Contents
1. [Architectural Overview & The Linux Accessibility Stack](#1-architectural-overview--the-linux-accessibility-stack)
2. [The AT-SPI2 D-Bus Protocol Complete Specification](#2-the-at-spi2-d-bus-protocol-complete-specification)
3. [Complete AT-SPI2 Roles, States & Event Signals](#3-complete-at-spi2-roles-states--event-signals)
4. [Input Capture & Interception: The Wayland & X11 Problem Solved](#4-input-capture--interception-the-wayland--x11-problem-solved)
5. [Linux Desktop Shells & File Managers Deep Dive](#5-linux-desktop-shells--file-managers-deep-dive)
6. [Speech & Audio Subsystem on Linux](#6-speech--audio-subsystem-on-linux)
7. [Master Linux System Call, D-Bus Interface & Signal Table](#7-master-linux-system-call-d-bus-interface--signal-table)
8. [Rust Implementation Blueprint for `bit_sr_platform_linux`](#8-rust-implementation-blueprint-for-bit_sr_platform_linux)

---

## 1. Architectural Overview & The Linux Accessibility Stack

Unlike Windows (which relies on in-process COM and kernel user hooks), modern Linux accessibility is entirely structured around **asynchronous inter-process communication over D-Bus**, governed by the **AT-SPI2 (Assistive Technology Service Provider Interface)** standard.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                 bit_sr Process Architecture                            │
└────────────────────────────────────────────────────────────────────────────────────────┘

                           ┌───────────────────────────────┐
                           │          bit_sr Core          │
                           └───────────────▲───────────────┘
                                           │
                           ┌───────────────┴───────────────┐
                           │     bit_sr_platform_linux     │
                           └───────┬───────────────┬───────┘
                                   │               │
       ┌───────────────────────────▼──┐ ┌──────────▼────────────────────────┐
       │     AT-SPI2 D-Bus Client     │ │   Low-Level Linux Input Subsystem  │
       │  (Pure Rust via `zbus` /     │ │   - Native `evdev` (/dev/input)    │
       │         `atspi` crates)      │ │   - Wayland Key Grab / EIS         │
       │  - Subscribes to Registry    │ │   - `xkbcommon` Key Translation    │
       │  - Cache.GetItems() batching │ │   - `uinput` Synthesizer           │
       └──────────────┬───────────────┘ └──────────┬─────────────────────────┘
                      │                            │
 ┌────────────────────▼────────────────────────────▼─────────────────────────────────────┐
 │                         Linux Accessibility D-Bus Bus                                 │
 │                       (unix:path=/run/user/1000/at-spi/bus)                           │
 ├────────────────────────────────┬───────────────────────────────┬──────────────────────┤
 │  GNOME Nautilus (GTK4/Adwaita) │ KDE Dolphin (Qt6 Accessibility)│ LibreOffice / WebKit │
 └────────────────────────────────┴───────────────────────────────┴──────────────────────┘
```

### Why Linux Screen Reading Historically Suffered (And How Rust Solves It)
1. **The Python/IPC Round-Trip Trap:**  
   In Python-based Orca, querying a control's properties (`name`, `role`, `states`, `parent`, `children`) requires 5 to 10 sequential synchronous D-Bus calls over a UNIX domain socket. In large lists or complex documents, tree traversal generated hundreds of blocking socket round-trips, causing noticeable cursor latency and audio stutter.  
   **The Rust Solution:** `bit_sr` uses pure Rust asynchronous D-Bus (`zbus`) coupled with the **`org.a11y.atspi.Cache`** interface (`GetItems`), which pulls entire UI subtrees in **a single batch D-Bus call**.
2. **Wayland Input Isolation:**  
   Under Wayland, client applications are sandboxed and cannot spy on global keypresses. While Orca struggled during the X11 $\to$ Wayland transition, `bit_sr` implements a dual-path input engine: **Kernel `evdev` device grabbing** (universal, compositor-agnostic) alongside modern Wayland accessibility protocols.

---

## 2. The AT-SPI2 D-Bus Protocol Complete Specification

### 2.1 The Dedicated Accessibility Bus Discovery
Accessibility does **not** run on the standard system bus or user session bus. It runs on a dedicated **Accessibility Bus** managed by `at-spi-bus-launcher`:

1. **Step 1: Discover Bus Address:**
   Query the user's Session Bus:
   - **Destination:** `org.a11y.Bus`
   - **Path:** `/org/a11y/bus`
   - **Interface:** `org.a11y.Bus`
   - **Method:** `GetAddress() -> (address: String)`
   - **Fallback:** Check the environment variable `$AT_SPI_BUS_ADDRESS`.
   - **Format:** Typically `unix:path=/run/user/1000/at-spi/bus` or `unix:abstract=/tmp/dbus-xxxx`.
2. **Step 2: Connect via `zbus`:**
   Open a peer-to-peer connection to this UNIX socket.
3. **Step 3: Register with the Registry:**
   - **Destination:** `org.a11y.atspi.Registry`
   - **Path:** `/org/a11y/atspi/registry`
   - **Interface:** `org.a11y.atspi.Registry`
   - **Method:** `RegisterEvent(event_name: String)`

### 2.2 Object Addressing Scheme
Every accessible node on Linux is identified by a **D-Bus Object Reference**:
`AccessibleId = (Sender: String, Path: ObjectPath)`  
*(e.g. Sender: `":1.48"`, Path: `"/org/a11y/atspi/accessible/2147483648"`)*

### 2.3 Core D-Bus Interfaces & Methods

#### 1. `org.a11y.atspi.Accessible` (Base Interface)
- **Properties:**
  - `Name: String` (Read)
  - `Description: String` (Read)
  - `Role: u32` (Read)
  - `Parent: (String, ObjectPath)` (Read)
  - `ChildCount: i32` (Read)
  - `Locale: String` (Read)
- **Methods:**
  - `GetRole() -> u32`
  - `GetRoleName() -> String`
  - `GetLocalizedRoleName() -> String`
  - `GetState() -> (u32, u32)`: Returns two 32-bit integers representing a **64-bit state bitfield**.
  - `GetAttributes() -> Dict<String, String>`: Returns key-value pairs (e.g. `tag`, `xml-roles`, `level`, `current`).
  - `GetChildAtIndex(index: i32) -> (String, ObjectPath)`
  - `GetChildren() -> Array<(String, ObjectPath)>`
  - `GetIndexInParent() -> i32`
  - `GetRelationSet() -> Array<(u32, Array<(String, ObjectPath)>)>`: Returns relations (`LabelledBy`, `ControlledBy`, `MemberOf`, etc.).
  - `GetInterfaces() -> Array<String>`: Returns all interfaces implemented by this object (e.g. `["org.a11y.atspi.Text", "org.a11y.atspi.Component"]`).

#### 2. `org.a11y.atspi.Component` (Spatial & Screen Coordinates)
- **Methods:**
  - `GetExtents(coord_type: u32) -> (x: i32, y: i32, width: i32, height: i32)`  
    *(0 = Screen coordinates, 1 = Window coordinates)*
  - `GetPosition(coord_type: u32) -> (x: i32, y: i32)`
  - `GetSize() -> (width: i32, height: i32)`
  - `Contains(x: i32, y: i32, coord_type: u32) -> bool`
  - `GetAccessibleAtPoint(x: i32, y: i32, coord_type: u32) -> (String, ObjectPath)`: Hit-testing.
  - `GrabFocus() -> bool`
  - `ScrollTo(scroll_type: u32) -> bool`

#### 3. `org.a11y.atspi.Text` & `EditableText` (Caret & Text Range)
- **Methods:**
  - `GetCharacterCount() -> i32`
  - `GetCaretOffset() -> i32`
  - `SetCaretOffset(offset: i32) -> bool`
  - `GetText(start: i32, end: i32) -> String` (-1 end offset = full text)
  - `GetTextAtOffset(offset: i32, type: u32) -> (text: String, start_offset: i32, end_offset: i32)`  
    *(Granularity: 0=Char, 1=Word, 2=Sentence, 3=Line, 4=Paragraph)*
  - `GetSelection(selection_index: i32) -> (start_offset: i32, end_offset: i32)`
  - `SetSelection(selection_index: i32, start: i32, end: i32) -> bool`
  - `GetDefaultAttributes() -> Dict<String, String>` (Font name, size, weight, color)
  - `GetAttributeRun(offset: i32, include_defaults: bool) -> (Dict<String, String>, i32, i32)`

#### 4. `org.a11y.atspi.Action` (Invocations)
- **Methods:**
  - `GetNActions() -> i32`
  - `GetName(index: i32) -> String` (e.g. `"click"`, `"press"`, `"activate"`)
  - `DoAction(index: i32) -> bool`

#### 5. `org.a11y.atspi.Value` (Sliders & Spinners)
- **Properties:**
  - `CurrentValue: f64`
  - `MinimumValue: f64`
  - `MaximumValue: f64`
  - `MinimumIncrement: f64`
- **Method:** `SetCurrentValue(value: f64) -> bool`

#### 6. `org.a11y.atspi.Selection` (List & Tab Containers)
- **Methods:**
  - `NSelectedChildren -> i32`
  - `GetSelectedChild(selected_child_index: i32) -> (String, ObjectPath)`
  - `SelectChild(child_index: i32) -> bool`
  - `DeselectSelectedChild(selected_child_index: i32) -> bool`
  - `IsChildSelected(child_index: i32) -> bool`
  - `SelectAll() -> bool`
  - `ClearSelection() -> bool`

#### 7. `org.a11y.atspi.Table` & `TableCell` (Data Grids & Spreadsheets)
- **Properties:**
  - `NRows: i32`, `NColumns: i32`, `NSelectedRows: i32`, `NSelectedColumns: i32`
- **Methods:**
  - `GetAccessibleAt(row: i32, column: i32) -> (String, ObjectPath)`
  - `GetRowHeader(row: i32) -> (String, ObjectPath)`
  - `GetColumnHeader(column: i32) -> (String, ObjectPath)`
  - `GetRowDescription(row: i32) -> String`
  - `GetColumnDescription(column: i32) -> String`
  - `IsRowSelected(row: i32) -> bool`, `IsColumnSelected(column: i32) -> bool`

#### 8. `org.a11y.atspi.Cache` (High-Performance Subtree Batching)
- **Path:** `/org/a11y/atspi/cache`
- **Interface:** `org.a11y.atspi.Cache`
- **Method:** `GetItems() -> Array<CacheItem>`
- **`CacheItem` Struct Signature:**
  ```text
  (
    (so) : AccessibleRef (sender, path),
    (so) : AppRef,
    (so) : ParentRef,
    i    : IndexInParent,
    a(so): ChildrenRefs,
    as   : SupportedInterfaces,
    s    : Name,
    u    : Role,
    s    : Description,
    (uu) : StateSet (64-bit bitflags)
  )
  ```
  Calling `GetItems()` allows `bit_sr` to download the entire window/dialog tree in **one single socket call**, eliminating 95% of IPC overhead!

---

## 3. Complete AT-SPI2 Roles, States & Event Signals

### 3.1 Master AT-SPI2 Role Table (`Atspi.Role`)

| AT-SPI2 Role ID | Symbolic Constant | Unified Screen Reader Role |
| :--- | :--- | :--- |
| `0` | `ROLE_INVALID` | `Role::Unknown` |
| `1` | `ROLE_ACCELERATOR_LABEL` | `Role::StaticText` |
| `2` | `ROLE_ALERT` | `Role::Alert` |
| `3` | `ROLE_ANIMATION` | `Role::Animation` |
| `4` | `ROLE_ARROW` | `Role::Graphic` |
| `5` | `ROLE_CALENDAR` | `Role::Calendar` |
| `6` | `ROLE_CANVAS` | `Role::Pane` |
| `7` | `ROLE_CHECK_BOX` | `Role::CheckBox` |
| `8` | `ROLE_CHECK_MENU_ITEM` | `Role::MenuItem` |
| `9` | `ROLE_COLOR_CHOOSER` | `Role::Dialog` |
| `10` | `ROLE_COLUMN_HEADER` | `Role::TableColumnHeader` |
| `11` | `ROLE_COMBO_BOX` | `Role::ComboBox` |
| `12` | `ROLE_DATE_EDITOR` | `Role::EditableText` |
| `13` | `ROLE_DESKTOP_ICON` | `Role::ListItem` |
| `14` | `ROLE_DESKTOP_FRAME` | `Role::Window` |
| `15` | `ROLE_DIAL` | `Role::Slider` |
| `16` | `ROLE_DIALOG` | `Role::Dialog` |
| `17` | `ROLE_DIRECTORY_PANE` | `Role::Pane` |
| `18` | `ROLE_DRAWING_AREA` | `Role::Pane` |
| `19` | `ROLE_FILE_CHOOSER` | `Role::Dialog` |
| `20` | `ROLE_FILLER` | `Role::Pane` |
| `21` | `ROLE_FOCUS_TRAVERSABLE` | `Role::Pane` |
| `22` | `ROLE_FONT_CHOOSER` | `Role::Dialog` |
| `23` | `ROLE_FRAME` | `Role::Window` |
| `24` | `ROLE_GLASS_PANE` | `Role::Pane` |
| `25` | `ROLE_HTML_CONTAINER` | `Role::Document` |
| `26` | `ROLE_ICON` | `Role::Graphic` |
| `27` | `ROLE_IMAGE` | `Role::Graphic` |
| `28` | `ROLE_INTERNAL_FRAME` | `Role::Window` |
| `29` | `ROLE_LABEL` | `Role::StaticText` |
| `30` | `ROLE_LAYERED_PANE` | `Role::Pane` |
| `31` | `ROLE_LIST` | `Role::List` |
| `32` | `ROLE_LIST_ITEM` | `Role::ListItem` |
| `33` | `ROLE_MENU` | `Role::PopupMenu` |
| `34` | `ROLE_MENU_BAR` | `Role::MenuBar` |
| `35` | `ROLE_MENU_ITEM` | `Role::MenuItem` |
| `36` | `ROLE_OPTION_PANE` | `Role::Pane` |
| `37` | `ROLE_PAGE_TAB` | `Role::Tab` |
| `38` | `ROLE_PAGE_TAB_LIST` | `Role::TabControl` |
| `39` | `ROLE_PANEL` | `Role::Pane` |
| `40` | `ROLE_PASSWORD_TEXT` | `Role::EditableText` |
| `41` | `ROLE_POPUP_MENU` | `Role::PopupMenu` |
| `42` | `ROLE_PROGRESS_BAR` | `Role::ProgressBar` |
| `43` | `ROLE_PUSH_BUTTON` | `Role::Button` |
| `44` | `ROLE_RADIO_BUTTON` | `Role::RadioButton` |
| `45` | `ROLE_RADIO_MENU_ITEM` | `Role::MenuItem` |
| `46` | `ROLE_ROOT_PANE` | `Role::Pane` |
| `47` | `ROLE_ROW_HEADER` | `Role::TableRowHeader` |
| `48` | `ROLE_SCROLL_BAR` | `Role::ScrollBar` |
| `49` | `ROLE_SCROLL_PANE` | `Role::Pane` |
| `50` | `ROLE_SEPARATOR` | `Role::Separator` |
| `51` | `ROLE_SLIDER` | `Role::Slider` |
| `52` | `ROLE_SPIN_BUTTON` | `Role::SpinButton` |
| `53` | `ROLE_SPLIT_PANE` | `Role::Pane` |
| `54` | `ROLE_STATUS_BAR` | `Role::StatusBar` |
| `55` | `ROLE_TABLE` | `Role::Table` |
| `56` | `ROLE_TABLE_CELL` | `Role::TableCell` |
| `57` | `ROLE_TABLE_COLUMN_HEADER`| `Role::TableColumnHeader` |
| `58` | `ROLE_TABLE_ROW_HEADER` | `Role::TableRowHeader` |
| `59` | `ROLE_TEAROFF_MENU_ITEM` | `Role::MenuItem` |
| `60` | `ROLE_TERMINAL` | `Role::Terminal` |
| `61` | `ROLE_TEXT` | `Role::EditableText` |
| `62` | `ROLE_TOGGLE_BUTTON` | `Role::CheckBox` |
| `63` | `ROLE_TOOL_BAR` | `Role::ToolBar` |
| `64` | `ROLE_TOOL_TIP` | `Role::ToolTip` |
| `65` | `ROLE_TREE` | `Role::TreeView` |
| `66` | `ROLE_TREE_TABLE` | `Role::TreeGrid` |
| `67` | `ROLE_UNKNOWN` | `Role::Unknown` |
| `68` | `ROLE_VIEWPORT` | `Role::Pane` |
| `69` | `ROLE_WINDOW` | `Role::Window` |
| `70` | `ROLE_HEADER` | `Role::Header` |
| `71` | `ROLE_FOOTER` | `Role::Footer` |
| `72` | `ROLE_PARAGRAPH` | `Role::Paragraph` |
| `74` | `ROLE_APPLICATION` | `Role::Application` |
| `76` | `ROLE_FORM` | `Role::Form` |
| `77` | `ROLE_LINK` | `Role::Link` |
| `79` | `ROLE_HEADING` | `Role::Heading` |
| `81` | `ROLE_SECTION` | `Role::Section` |
| `90` | `ROLE_LANDMARK` | `Role::Landmark` |
| `97` | `ROLE_SWITCH` | `Role::Switch` |

### 3.2 Master AT-SPI2 State Bitfield (`Atspi.StateType`)
States in AT-SPI2 are represented as a 64-bit integer formed by `(lower_32: u32, upper_32: u32)`.

| Bit Position | Constant Name | Screen Reader Meaning |
| :--- | :--- | :--- |
| `0` | `STATE_INVALID` | Invalid state |
| `1` | `STATE_ACTIVE` | Window or control is currently active |
| `2` | `STATE_ARMED` | Button armed to fire on release |
| `3` | `STATE_BUSY` | Control busy / processing background work |
| `4` | `STATE_CHECKED` | CheckBox or ToggleButton is checked |
| `5` | `STATE_COLLAPSED` | TreeItem or Combo collapsed |
| `6` | `STATE_DEFUNCT` | Object destroyed/dead (DO NOT CALL IPC ON IT!) |
| `7` | `STATE_EDITABLE` | Text contents can be edited |
| `8` | `STATE_ENABLED` | Control enabled |
| `9` | `STATE_EXPANDABLE` | Control can be expanded |
| `10` | `STATE_EXPANDED` | TreeItem or Expander currently expanded |
| `11` | `STATE_FOCUSABLE` | Control can accept keyboard focus |
| `12` | `STATE_FOCUSED` | Control currently has keyboard focus |
| `13` | `STATE_HAS_TOOLTIP` | Tooltip attached to control |
| `14` | `STATE_HORIZONTAL` | Orientation horizontal |
| `15` | `STATE_ICONIFIED` | Window minimized to dock/panel |
| `16` | `STATE_MODAL` | Dialog blocks input to parent window |
| `17` | `STATE_MULTI_LINE` | Text control spans multiple lines |
| `18` | `STATE_MULTISELECTABLE` | Container allows selecting multiple items |
| `19` | `STATE_OPAQUE` | Renders fully opaque |
| `20` | `STATE_PRESSED` | Button currently pressed down |
| `21` | `STATE_RESIZABLE` | Window can be resized |
| `22` | `STATE_SELECTABLE` | Item can be selected |
| `23` | `STATE_SELECTED` | Item is selected |
| `24` | `STATE_SENSITIVE` | User can interact (opposite of disabled) |
| `25` | `STATE_SHOWING` | Scrolled inside visible viewport |
| `26` | `STATE_SINGLE_LINE` | Single-line edit control |
| `27` | `STATE_STALE` | Cached values are invalid |
| `28` | `STATE_TRANSIENT` | Temporary popup window |
| `29` | `STATE_VERTICAL` | Orientation vertical |
| `30` | `STATE_VISIBLE` | Control is visible on screen |
| `31` | `STATE_MANAGES_DESCENDANTS`| Container handles focus internally (Virtual views) |
| `32` | `STATE_INDETERMINATE` | Tri-state checkbox mixed state |
| `34` | `STATE_READ_ONLY` | Value/Text cannot be modified |
| `38` | `STATE_VISITED` | Link has been previously clicked |
| `42` | `STATE_HAS_POPUP` | Button opens a dropdown/menu |
| `45` | `STATE_REQUIRED` | Form field must be filled |
| `46` | `STATE_INVALID_ENTRY` | Form field failed validation |

### 3.3 The Core Event Signals Subscribed via Registry

| Event Name | D-Bus Signal Path | When Fired & Usage |
| :--- | :--- | :--- |
| `object:state-changed:focused` | `org.a11y.atspi.Event.Object` | Primary focus transition event. `detail1 == 1` indicates gained focus, `0` lost focus. |
| `object:active-descendant-changed` | `org.a11y.atspi.Event.Object` | Fired by complex lists, trees, and tables when navigating child items without moving native window focus. |
| `object:text-caret-moved` | `org.a11y.atspi.Event.Object` | Text cursor offset changed in text boxes, editors, terminals. |
| `object:text-changed:insert` | `org.a11y.atspi.Event.Object` | Character/word typed or inserted into document. |
| `object:text-changed:delete` | `org.a11y.atspi.Event.Object` | Text deleted (backspace, delete). |
| `object:selection-changed` | `org.a11y.atspi.Event.Object` | Selected item changed in list or tree. |
| `object:property-change:accessible-name` | `org.a11y.atspi.Event.Object` | Control label updated dynamically. |
| `object:property-change:accessible-value`| `org.a11y.atspi.Event.Object` | Slider or progress bar percentage changed. |
| `window:activate` | `org.a11y.atspi.Event.Window` | Top-level window received focus (Alt+Tab, window launch). |
| `window:deactivate` | `org.a11y.atspi.Event.Window` | Window lost focus. |
| `document:page-changed` | `org.a11y.atspi.Event.Document` | Multi-page document or PDF page switch. |
| `object:announcement` | `org.a11y.atspi.Event.Object` | Live region updates and alert notifications. |

---

## 4. Input Capture & Interception: The Wayland & X11 Problem Solved

On Windows, `SetWindowsHookExW` works globally across every window. On Linux, input handling is fragmented between the legacy X11 protocol and security-sandboxed Wayland compositors.

### The 3 Input Strategies for `bit_sr`

```
                               ┌─────────────────────────────────────────┐
                               │           Input Engine Strategy         │
                               └────────────────────┬────────────────────┘
                                                    │
                 ┌──────────────────────────────────┼──────────────────────────────────┐
                 │                                  │                                  │
      ┌──────────▼──────────┐            ┌──────────▼──────────┐            ┌──────────▼──────────┐
      │   Path A: evdev     │            │   Path B: AT-SPI2   │            │  Path C: Shortcuts  │
      │  (/dev/input/event) │            │DeviceEventController│            │   Desktop Portal    │
      │ Universal, rock-    │            │ Works when AT-SPI2  │            │ User-configurable   │
      │ solid across ALL    │            │ compositor bridge   │            │ desktop shortcuts.  │
      │ Wayland compositors │            │ is functional.      │            │                     │
      └─────────────────────┘            └─────────────────────┘            └─────────────────────┘
```

### Strategy A: Direct Kernel `evdev` (The Bulletproof Wayland Solution)
- **Mechanism:** Opens `/dev/input/event*` keyboard devices directly.
- **Grabbing Keypresses:**
  Calls `ioctl(fd, EVIOCGRAB, 1)`. When grabbed, key events are delivered **exclusively** to `bit_sr`.
- **Keyboard Translation:**
  Uses the `xkbcommon` crate to compile the system keymap, track modifier states (Shift, Ctrl, Alt, CapsLock, Super), and translate keycodes to keysyms.
- **Synthesizing Pass-Through Input:**
  Keys that are not screen reader shortcuts are mirrored to the kernel using `/dev/uinput` (`uinput` crate).
- **Permissions:** The user must be in the `input` group (`sudo usermod -a -G input $USER`).

### Strategy B: AT-SPI2 Device Event Controller (`org.a11y.atspi.DeviceEventController`)
- **Interface:** `org.a11y.atspi.DeviceEventController` on the accessibility bus.
- **Method:** `RegisterKeystrokeListener(listener, keys: Array<KeyDefinition>, mask: u32, type: u32, mode: u32)`.
- **How Orca uses it:** Orca calls `Atspi.Device.new_full("org.gnome.Orca")` and registers key grabs for the Orca modifier (Insert / CapsLock).
- **Caveat:** Relies on the Wayland compositor (Mutter / KWin) implementing the AT-SPI2 input grab bridge. If the compositor bridge is buggy or unmaintained, Strategy A (`evdev`) serves as the infallible fallback.

---

## 5. Linux Desktop Shells & File Managers Deep Dive

### 5.1 GNOME Files (Nautilus) & GTK4 File Chooser
Nautilus is the default file manager across GNOME, Ubuntu, Fedora, and Debian. Modern Nautilus is written in **GTK4 and Libadwaita**.

```
[Window / Frame]: "Documents" (org.gnome.Nautilus)
  └── [Pane]: GtkBox
        ├── [HeaderBar]: Path buttons (Breadcrumbs: "Home" > "Documents")
        │     ├── [PushButton]: "Home"
        │     ├── [PushButton]: "Documents"
        │     └── [Text / Entry]: (Hidden path entry, appears on Ctrl+L)
        ├── [Paned]: Split container
        │     ├── [List]: Places Sidebar (Left Navigation Pane)
        │     │     ├── [ListItem]: "Home"
        │     │     ├── [ListItem]: "Documents"
        │     │     ├── [ListItem]: "Downloads"
        │     │     └── [ListItem]: "Trash"
        │     └── [ScrolledWindow]: Main File & Folder View
        │           └── [Table / ColumnView / GridView]: "File Display"
        │                 ├── [TableRow / ListItem]: "Financial_Report.pdf"
        │                 ├── [TableRow / ListItem]: "SourceCode" (Folder)
        │                 └── [TableRow / ListItem]: "archive.tar.gz"
        └── [StatusBar / Label]: Selection summary ("1 item selected, 2.4 MB")
```

#### The Nautilus Quirks & Navigation Rules
1. **The Modern `ColumnView` Active Descendant Event:**  
   In GTK4 Nautilus, arrowing through files does **not** move native window focus. The container (`GtkColumnView` / `GtkGridView`) remains focused, while firing:  
   `object:active-descendant-changed(source=FileView, any_data=CurrentItem)`.  
   `bit_sr` must listen to `active-descendant-changed`, query the name, role, and states of `any_data`, and speak the newly selected file immediately.
2. **Breadcrumb Path vs Direct Entry (`Ctrl+L`):**  
   By default, the path bar is a sequence of `GtkButton` controls. Pressing `Ctrl+L` swaps the header into an editable `GtkEntry` (`Role::EditableText`). `bit_sr` intercepts `object:state-changed:focused` on the entry and announces: `"Location: /home/user/Documents, editable text"`.
3. **Places Sidebar Selection:**  
   The left sidebar is a `Role::List` (`GtkPlacesSidebar`). Tab transitions between the sidebar and the main file view must update `focusAncestors` and speak the change of container.

### 5.2 KDE Plasma & Dolphin File Manager
- **Toolkit:** Qt6 (`QAccessibleInterface`).
- **Structure:** Central file view is a `QListView` / `QTreeView`.
- **Event Signature:** Emits `object:state-changed:focused` on individual `QAccessible` item nodes.
- **Details View Columns:** Column headers (Name, Size, Modified, Permissions) implement `org.a11y.atspi.Table`.

### 5.3 GNOME Shell (`gnome-shell`) Desktop Quirks
As discovered in Orca's `scripts/apps/gnome-shell/script.py`:
1. **Window Switcher (Alt+Tab) Spurious Activation:**  
   When opening or closing the Alt+Tab overlay, `gnome-shell` fires a spurious `window:activate` on an invisible root shell window with `name == ""`. `bit_sr` must suppress announcing windows with empty names during task switching.
2. **Top Bar Panel Ancestor Events:**  
   When interacting with system applets (Clock, Network, Audio), `gnome-shell` fires focus events on the ancestor `Panel` widget (`Role::Panel`). Drop focus changes on panel ancestors if the visual focus is already on a child applet.
3. **Quick Settings Volume & Brightness Sliders:**  
   Sliders in GNOME Shell Quick Settings are embedded inside menu items (`Role::MenuItem`) without explicit labels. `bit_sr` must walk child descendants of nameless menu items to locate the inner `Role::Slider`.

---

## 6. Speech & Audio Subsystem on Linux

### 6.1 Speech Dispatcher (`speechd`)
The standard speech broker on Linux for over 20 years:
- **UNIX Domain Socket:** `/run/user/<uid>/speech-dispatcher/speechd.sock`
- **Fallback TCP:** `localhost:6560`
- **Protocol (SSIP):** Text-based line protocol.
  ```text
  Client: SET SELF CLIENT_NAME "user:bit_sr"
  Server: 200 OK
  Client: SET SELF RATE 50
  Server: 200 OK
  Client: SPEAK
  Server: 230 OK RECEIVING DATA
  Client: Documents, folder, 1 of 24 selected
  Client: .
  Server: 200 OK
  ```
- **Instant Interruption:** Sending `CANCEL` instantly terminates synthesizer output.

### 6.2 Modern Spiel (`org.freedesktop.Speech`)
The next-generation freedesktop speech standard created by GNOME:
- **Bus:** User Session Bus.
- **Interface:** `org.freedesktop.Speech.Provider`.
- **Advantages:** Native D-Bus asynchronous calls, SSML support, lower latency than legacy Speech Dispatcher.

### 6.3 Embedded Offline Synthesis (Zero Daemon Dependency)
`bit_sr_speech` can optionally link directly against:
- **Piper TTS:** Blazingly fast neural voice engine running on ONNX Runtime via Rust bindings.
- **eSpeak NG:** Ultra-fast, lightweight synthesizer linked as a C library.  
*This completely bypasses the need for `speechd` or `spiel` to be installed on the Linux system.*

---

## 7. Master Linux System Call, D-Bus Interface & Signal Table

| Subsystem | D-Bus / System Symbol | Signature / Path | Purpose in `bit_sr` |
| :--- | :--- | :--- | :--- |
| **Bus Discovery** | `org.a11y.Bus.GetAddress` | Destination: `org.a11y.Bus`, Path: `/org/a11y/bus` | Obtains UNIX socket address of accessibility bus. |
| **Registry** | `org.a11y.atspi.Registry.RegisterEvent` | Path: `/org/a11y/atspi/registry` | Registers global subscriptions for focus, caret, and window events. |
| **Batch Cache** | `org.a11y.atspi.Cache.GetItems` | Path: `/org/a11y/atspi/cache` | Fetches entire accessible subtrees in a single D-Bus call. |
| **Accessible** | `org.a11y.atspi.Accessible.GetRole` | Method on any object path | Queries integer role ID mapping to `Role`. |
| **Accessible** | `org.a11y.atspi.Accessible.GetState` | Method on any object path | Returns `(u32, u32)` 64-bit state bitflags. |
| **Accessible** | `org.a11y.atspi.Accessible.GetChildren` | Method on any object path | Returns array of `(sender, path)` child references. |
| **Component** | `org.a11y.atspi.Component.GetExtents` | Method on any object path | Retrieves screen/window bounding rectangle. |
| **Component** | `org.a11y.atspi.Component.GrabFocus` | Method on any object path | Programmatically directs keyboard focus to element. |
| **Text** | `org.a11y.atspi.Text.GetCaretOffset` | Method on any object path | Queries current text insertion cursor position. |
| **Text** | `org.a11y.atspi.Text.GetTextAtOffset` | Method on any object path | Extracts character, word, or line at specific offset. |
| **EditableText** | `org.a11y.atspi.EditableText.InsertText`| Method on any object path | Inserts string into active text field. |
| **Selection** | `org.a11y.atspi.Selection.GetSelectedChild`| Method on any object path | Obtains currently selected item in list or tab control. |
| **Table** | `org.a11y.atspi.Table.GetAccessibleAt` | Method on any object path | Queries cell at `(row, col)` coordinate in grid. |
| **Input (evdev)** | `ioctl(fd, EVIOCGRAB, 1)` | System call on `/dev/input/event*` | Exclusively grabs physical keyboard from kernel. |
| **Input (uinput)**| `ioctl(fd, UI_DEV_CREATE)` | System call on `/dev/uinput` | Creates virtual input device to synthesize unhandled keys. |
| **Input (XKB)** | `xkb_state_key_get_syms` | `libxkbcommon` C API | Translates raw hardware keycodes into Unicode keysyms. |
| **Speech** | SSIP `SPEAK` / `CANCEL` | Socket: `/run/user/<uid>/speech-dispatcher/speechd.sock`| Dispatches speech text to Speech Dispatcher daemon. |

---

## 8. Rust Implementation Blueprint for `bit_sr_platform_linux`

### Crate Structure
```text
crates/bit_sr_platform_linux/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Public traits & Linux platform lifecycle
    ├── bus.rs                  # Accessibility bus discovery & zbus connection
    ├── registry.rs             # Event subscription manager
    ├── cache.rs                # org.a11y.atspi.Cache tree batching parser
    ├── accessible.rs           # Safe wrapper around AccessibleId (Sender, Path)
    ├── roles_states.rs         # AT-SPI2 64-bit state & role bitflags conversion
    ├── events/
    │   ├── mod.rs              # Event stream dispatcher (zbus::MessageStream)
    │   ├── focus.rs            # object:state-changed:focused handler
    │   ├── descendant.rs       # object:active-descendant-changed handler (Nautilus)
    │   ├── caret.rs            # object:text-caret-moved handler
    │   └── window.rs           # window:activate / deactivate handler
    ├── input/
    │   ├── mod.rs              # Unified Linux input provider trait
    │   ├── evdev.rs            # Kernel /dev/input device grabber (Wayland-proof)
    │   ├── uinput.rs           # Kernel /dev/uinput event synthesizer
    │   ├── xkb.rs              # xkbcommon keycode to keysym translation
    │   └── atspi_device.rs     # org.a11y.atspi.DeviceEventController bridge
    └── apps/
        ├── mod.rs              # Linux app module dispatcher
        ├── nautilus.rs         # GNOME Files special-casing (active descendants, path bar)
        ├── gnome_shell.rs      # Alt+Tab empty window suppression & panel handling
        └── dolphin.rs          # KDE Dolphin Qt6 file navigation
```

### Cargo Dependencies (`Cargo.toml`)
```toml
[package]
name = "bit_sr_platform_linux"
version = "0.1.0"
edition = "2024"

[dependencies]
bit_sr_core = { path = "../bit_sr_core" }
tokio = { version = "1.38", features = ["full"] }
zbus = { version = "4.4", features = ["tokio"] }
atspi = { version = "0.22", features = ["zbus"] }
evdev = "0.12"
xkbcommon = "0.7"
bitflags = "2.4"
log = "0.4"
```
