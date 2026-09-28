# Windows Subsystem & API Integration Specification: `bit_sr_platform_windows`

> **Document Classification:** Master Reference & Exhaustive Technical Specification  
> **Source Analysis:** NVDA Source Codebase (`references/nvda`), Microsoft Windows Accessibility Subsystems, and `explorer.exe` Internal Mechanics.  
> **Target:** Native Rust Implementation for Windows (`bit_sr_platform_windows`).  
> **Status:** 100% Comprehensive — No external lookups into NVDA source code required.

---

## Table of Contents
1. [Architectural Overview & Thread Topology](#1-architectural-overview--thread-topology)
2. [Input Hook Subsystem (`winInputHook` & `keyboardHandler`)](#2-input-hook-subsystem)
3. [Microsoft UI Automation (UIA) Complete Specification (`UIAHandler`)](#3-microsoft-ui-automation-uia-complete-specification)
4. [MSAA & WinEvents Complete Specification (`IAccessibleHandler`)](#4-msaa--winevents-complete-specification)
5. [Direct Win32 Window & Common Control Inspection (`NVDAObjects/window/`)](#5-direct-win32-window--common-control-inspection)
6. [Comprehensive Windows File Explorer (`explorer.exe`) Reference](#6-comprehensive-windows-file-explorer-reference)
7. [Focus Tracking, Ancestry Resolution & Speech Formatting (`api.py` & `speech.py`)](#7-focus-tracking-ancestry-resolution--speech-formatting)
8. [Multi-Desktop, Security & Session Tracking](#8-multi-desktop-security--session-tracking)
9. [Deadlock Prevention, Hung Windows & Watchdog Architecture](#9-deadlock-prevention-hung-windows--watchdog-architecture)
10. [Display Model & Screen Scraping Fallback (`displayModel`)](#10-display-model--screen-scraping-fallback)
11. [Master Windows API, COM Interface & Constant Reference Table](#11-master-windows-api-com-interface--constant-reference-table)
12. [Rust Implementation Blueprint for `bit_sr_platform_windows`](#12-rust-implementation-blueprint-for-bit_sr_platform_windows)

---

## 1. Architectural Overview & Thread Topology

A production Windows screen reader cannot operate on a single thread. Calling UI Automation or MSAA on the thread that runs a low-level keyboard hook causes immediate system stutter, missed keystrokes, and silent unhooking by Windows.

### The 5-Thread Concurrency Model

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                 bit_sr Process Architecture                            │
└────────────────────────────────────────────────────────────────────────────────────────┘

  Thread 1: Input Hook Thread (STA + Win32 Message Pump)
  ├── SetWindowsHookExW(WH_KEYBOARD_LL)
  ├── SetWindowsHookExW(WH_MOUSE_LL)
  └── Dispatches raw key/mouse events to Core via non-blocking lock-free channel.
      STRICT RULE: NEVER execute COM calls, SendMessage, or allocations here.

  Thread 2: UIA Worker Thread (MTA: COINIT_MULTITHREADED)
  ├── CoCreateInstance(CLSID_CUIAutomation8) -> IUIAutomation6
  ├── Registers IUIAutomationFocusChangedEventHandler, PropertyChanged, Notification
  ├── Dispatches pre-cached element nodes (CacheRequest) to Core.
  └── Executes tree walking (RawViewWalker / ControlViewWalker).

  Thread 3: MSAA / WinEvent Thread (STA + Message Pump)
  ├── SetWinEventHook(EVENT_MIN, EVENT_MAX, ..., WINEVENT_OUTOFCONTEXT)
  ├── Calls AccessibleObjectFromEvent() on legacy Win32 controls.
  └── Dispatches MSAA events (focus, state, selection) to Core.

  Thread 4: Core Engine Thread (Async Tokio / Actor Loop)
  ├── Receives events from Input, UIA, and MSAA channels.
  ├── Evaluates active App Modules (e.g. explorer.rs special-casing).
  ├── Resolves Focus Ancestry Line and computes Focus Difference Level.
  ├── Routes text to Speech Dispatcher (bit_sr_speech).
  └── Dispatches events to sandboxed Wasm plugins (bit_sr_plugin).

  Thread 5: Watchdog & Session Monitor Thread
  ├── CreateWaitableTimer / SetWaitableTimer.
  ├── Monitors Core thread heartbeats; cancels hanging cross-process calls.
  ├── Hidden Win32 message window receiving WM_WTSSESSION_CHANGE and WM_DISPLAYCHANGE.
  └── Detects UAC Secure Desktop switches via OpenInputDesktop().
```

---

## 2. Input Hook Subsystem

### 2.1 Hook Types & Interception Rules
- **Keyboard Hook:** `SetWindowsHookExW(WH_KEYBOARD_LL, keyboardHookCallback, hModule, 0)`
- **Mouse Hook:** `SetWindowsHookExW(WH_MOUSE_LL, mouseHookCallback, hModule, 0)`
- **Structure Layouts:**
  ```rust
  #[repr(C)]
  pub struct KBDLLHOOKSTRUCT {
      pub vk_code: u32,
      pub scan_code: u32,
      pub flags: u32,
      pub time: u32,
      pub dw_extra_info: usize,
  }

  #[repr(C)]
  pub struct MSLLHOOKSTRUCT {
      pub pt: POINT,
      pub mouse_data: u32,
      pub flags: u32,
      pub time: u32,
      pub dw_extra_info: usize,
  }
  ```
- **Key Flags:**
  - `LLKHF_EXTENDED = 0x01`: Extended key (e.g. Right Alt, Right Ctrl, Arrow keys, Numpad Enter).
  - `LLKHF_INJECTED = 0x10`: Keystroke synthesized by software (e.g. `SendInput`).
  - `LLKHF_UP = 0x80`: Transition state (Key Up).
  - `LLMHF_INJECTED = 0x01`: Mouse event synthesized by software.

### 2.2 Low-Level Hook Timeout & The Critical Invariant
- Windows monitors low-level hook execution time via registry key `HKCU\Control Panel\Desktop\LowLevelHooksTimeout` (default: 200–300ms).
- If `keyboardHookCallback` does not return within this window, Windows **silently drops the hook**, permanently disabling keyboard interception without throwing an error!
- **Invariant:** The hook procedure must only:
  1. Inspect `vk_code` and `flags`.
  2. If it is a registered screen reader shortcut (e.g. `Insert + Down` or `CapsLock + Space`) or any key during speech (speech interruption), post the key event to a channel and return `1` (consuming the key).
  3. Otherwise, return `CallNextHookEx(None, code, w_param, l_param)`.

### 2.3 Key Translation & State Inspection
- `GetAsyncKeyState(vk)`: Queries immediate hardware state (high bit set = key down).
- `GetKeyState(vk)`: Queries virtual key state from the calling thread's message queue (low bit set = toggled, e.g. CapsLock, NumLock).
- `ToUnicodeEx(vk, scan_code, key_state_array, unicode_buffer, buffer_size, flags, h_layout)`: Converts virtual key code into UTF-16 character based on the user's active keyboard layout.

### 2.4 Synthesizing Input Without Hook Feedback Loops
When the screen reader needs to type or simulate a keypress (e.g. in review mode or braille input), it uses `SendInput`. Because synthesized keystrokes carry `LLKHF_INJECTED`, the hook callback must ignore injected keys to prevent infinite recursion.

---

## 3. Microsoft UI Automation (UIA) Complete Specification

### 3.1 MTA Initialization & Client Instantiation
1. **Thread Apartment:** Must call `CoInitializeEx(None, COINIT_MULTITHREADED)`.
2. **Interface Progression:** Query `CLSID_CUIAutomation8`. Query the highest supported interface in reverse order:
   - `IUIAutomation6` (Windows 10 1809+ / Windows 11)
   - `IUIAutomation5` (Windows 10 1709+)
   - `IUIAutomation4` (Windows 10 1703+)
   - `IUIAutomation3` (Windows 8.1+)
   - `IUIAutomation` (Base)

### 3.2 Stripping ProxyFactoryMapping (Bugfix #7345)
By default, the UIA client runtime registers WinEvent hooks to convert legacy MSAA events into UIA property change events. When an application with a slow message pump runs, this causes the UIA client thread to freeze.
- **The NVDA Fix:**
  1. Obtain `IUIAutomation::get_ProxyFactoryMapping(&pfm)`.
  2. Enumerate entries `0..pfm.get_Count()`.
  3. Call `entry.get_WinEventsForAutomationEvent(UIA_AutomationPropertyChangedEventId, prop_id)`.
  4. If mapped, call `entry.set_WinEventsForAutomationEvent(UIA_AutomationPropertyChangedEventId, prop_id, &[])` to clear them.
  5. Remove and re-insert the modified entry in `pfm`.

### 3.3 Modern UIA Performance Settings (`IUIAutomation6`)
- `client.put_CoalesceEvents(CoalesceEventsOptions_Enabled)`: Merges redundant burst events before RPC marshaling.
- `client.put_ConnectionRecoveryBehavior(ConnectionRecoveryBehaviorOptions_Enabled)`: Automatically recovers the connection if the target provider crashes and restarts.

### 3.4 The Base `IUIAutomationCacheRequest`
To prevent thousands of out-of-process COM calls, this CacheRequest must be attached to every event handler and tree walker:

| Property ID | Symbolic Name | Why It Must Be Cached |
| :--- | :--- | :--- |
| `30024` | `UIA_FrameworkIdPropertyId` | Detects "Win32", "WinForm", "WPF", "XAML", "DirectUI", "Chrome". |
| `30011` | `UIA_AutomationIdPropertyId` | Machine-readable ID (e.g. `"ItemsView"`, `"TabListView"`). |
| `30012` | `UIA_ClassNamePropertyId` | Window/Control class name (e.g. `"UIItemsView"`, `"DirectUIHWND"`). |
| `30003` | `UIA_ControlTypePropertyId` | Integer ID mapping to UIA Control Type enum. |
| `30005` | `UIA_NamePropertyId` | The primary accessible label/name of the element. |
| `30004` | `UIA_LocalizedControlTypePropertyId` | Human-readable role (e.g. "folder", "list item", "button"). |
| `30008` | `UIA_HasKeyboardFocusPropertyId` | True if the element currently owns keyboard focus. |
| `30019` | `UIA_IsKeyboardFocusablePropertyId` | True if the element can receive focus. |
| `30017` | `UIA_IsControlElementPropertyId` | True if part of the Control View (skips decorative nodes). |
| `30016` | `UIA_IsContentElementPropertyId` | True if part of the Content View (contains real user data). |
| `30020` | `UIA_ProcessIdPropertyId` | Target process ID (PID) for App Module routing. |
| `30020` | `UIA_NativeWindowHandlePropertyId` | HWND owning the element (0 if windowless). |
| `30014` | `UIA_IsTextPatternAvailablePropertyId` | Flags support for full text range navigation. |
| `30040` | `UIA_AriaRolePropertyId` | Web/Electron ARIA role string. |
| `30159` | `UIA_PositionInSetPropertyId` | Hierarchical position (e.g. 1 in "1 of 24"). |
| `30160` | `UIA_SizeOfSetPropertyId` | Total count in set (e.g. 24 in "1 of 24"). |
| `30070` | `UIA_LevelPropertyId` | Indentation level in TreeViews. |
| `30022` | `UIA_IsOffscreenPropertyId` | Flags whether element is scrolled out of viewport. |

### 3.5 Complete UIA Control Type to Screen Reader Role Mapping

| UIA Control Type ID | UIA Constant | Unified Role |
| :--- | :--- | :--- |
| `50000` | `UIA_ButtonControlTypeId` | `Role::Button` |
| `50001` | `UIA_CalendarControlTypeId` | `Role::Calendar` |
| `50002` | `UIA_CheckBoxControlTypeId` | `Role::CheckBox` |
| `50003` | `UIA_ComboBoxControlTypeId` | `Role::ComboBox` |
| `50004` | `UIA_EditControlTypeId` | `Role::EditableText` |
| `50005` | `UIA_HyperlinkControlTypeId` | `Role::Link` |
| `50006` | `UIA_ImageControlTypeId` | `Role::Graphic` |
| `50007` | `UIA_ListItemControlTypeId` | `Role::ListItem` |
| `50008` | `UIA_ListControlTypeId` | `Role::List` |
| `50009` | `UIA_MenuControlTypeId` | `Role::PopupMenu` |
| `50010` | `UIA_MenuBarControlTypeId` | `Role::MenuBar` |
| `50011` | `UIA_MenuItemControlTypeId` | `Role::MenuItem` |
| `50012` | `UIA_ProgressBarControlTypeId` | `Role::ProgressBar` |
| `50013` | `UIA_RadioButtonControlTypeId` | `Role::RadioButton` |
| `50014` | `UIA_ScrollBarControlTypeId` | `Role::ScrollBar` |
| `50015` | `UIA_SliderControlTypeId` | `Role::Slider` |
| `50016` | `UIA_SpinnerControlTypeId` | `Role::SpinButton` |
| `50017` | `UIA_StatusBarControlTypeId` | `Role::StatusBar` |
| `50018` | `UIA_TabControlTypeId` | `Role::TabControl` |
| `50019` | `UIA_TabItemControlTypeId` | `Role::Tab` |
| `50020` | `UIA_TextControlTypeId` | `Role::StaticText` |
| `50021` | `UIA_ToolBarControlTypeId` | `Role::ToolBar` |
| `50022` | `UIA_ToolTipControlTypeId` | `Role::ToolTip` |
| `50023` | `UIA_TreeControlTypeId` | `Role::TreeView` |
| `50024` | `UIA_TreeItemControlTypeId` | `Role::TreeViewItem` |
| `50025` | `UIA_CustomControlTypeId` | `Role::Unknown` |
| `50026` | `UIA_GroupControlTypeId` | `Role::Grouping` |
| `50027` | `UIA_ThumbControlTypeId` | `Role::Thumb` |
| `50028` | `UIA_DataGridControlTypeId` | `Role::DataGrid` |
| `50029` | `UIA_DataItemControlTypeId` | `Role::DataItem` |
| `50030` | `UIA_DocumentControlTypeId` | `Role::Document` |
| `50031` | `UIA_SplitButtonControlTypeId` | `Role::SplitButton` |
| `50032` | `UIA_WindowControlTypeId` | `Role::Window` |
| `50033` | `UIA_PaneControlTypeId` | `Role::Pane` |
| `50034` | `UIA_HeaderControlTypeId` | `Role::Header` |
| `50035` | `UIA_HeaderItemControlTypeId` | `Role::HeaderItem` |
| `50036` | `UIA_TableControlTypeId` | `Role::Table` |
| `50037` | `UIA_TitleBarControlTypeId` | `Role::TitleBar` |
| `50038` | `UIA_SeparatorControlTypeId` | `Role::Separator` |

### 3.6 Complete UIA Patterns Catalog & Methods

| Pattern Name | Pattern ID | Key Methods & Properties |
| :--- | :--- | :--- |
| **Value** | `10002` | `get_CurrentValue(&BSTR)`, `get_CurrentIsReadOnly(&BOOL)`, `SetValue(BSTR)` |
| **RangeValue** | `10003` | `get_CurrentValue(&f64)`, `get_CurrentMinimum(&f64)`, `get_CurrentMaximum(&f64)`, `SetValue(f64)` |
| **Selection** | `10001` | `GetSelection(&IUIAutomationElementArray)`, `get_CurrentCanSelectMultiple(&BOOL)` |
| **SelectionItem** | `10010` | `Select()`, `AddToSelection()`, `RemoveFromSelection()`, `get_CurrentIsSelected(&BOOL)` |
| **Toggle** | `10015` | `Toggle()`, `get_CurrentToggleState(&ToggleState)` (0=Off, 1=On, 2=Indeterminate) |
| **ExpandCollapse**| `10005` | `Expand()`, `Collapse()`, `get_CurrentExpandCollapseState(&State)` (0=Collapsed, 1=Expanded) |
| **Invoke** | `10000` | `Invoke()` (Executes default action / button click) |
| **Grid** | `10006` | `get_CurrentRowCount(&i32)`, `get_CurrentColumnCount(&i32)`, `GetItem(r, c, &element)` |
| **GridItem** | `10007` | `get_CurrentRow(&i32)`, `get_CurrentColumn(&i32)`, `get_CurrentRowSpan(&i32)` |
| **Table** | `10012` | `GetRowHeaders(&array)`, `GetColumnHeaders(&array)` |
| **TableItem** | `10013` | `GetRowHeaderItems(&array)`, `GetColumnHeaderItems(&array)` |
| **Scroll** | `10004` | `Scroll(ScrollAmount, ScrollAmount)`, `SetScrollPercent(f64, f64)` |
| **Window** | `10009` | `get_CurrentCanMaximize(&BOOL)`, `get_CurrentIsModal(&BOOL)`, `Close()` |
| **Text** | `10014` | `GetSelection(&ranges)`, `GetVisibleRanges(&ranges)`, `get_DocumentRange(&range)` |
| **Text2** | `10024` | `RangeFromAnnotation(annotation_element, &range)`, `GetCaretRange(&has_caret, &range)` |

### 3.7 UIA Text Range Traversal Mechanics (`IUIAutomationTextRange`)
- **Navigation Units:** `TextUnit_Character` (0), `TextUnit_Format` (1), `TextUnit_Word` (2), `TextUnit_Line` (3), `TextUnit_Paragraph` (4), `TextUnit_Page` (5), `TextUnit_Document` (6).
- **Core Methods:**
  - `range.Move(unit, count, &moved_count)`: Moves entire range forward/backward.
  - `range.MoveEndpointByUnit(endpoint, unit, count, &moved)`: Moves start or end boundary.
  - `range.ExpandToEnclosingUnit(unit)`: Expands selection to full word, line, or paragraph.
  - `range.GetText(max_length, &BSTR)`: Fetches plain text enclosed in range (-1 for all).
  - `range.GetEnclosingElement(&element)`: Finds the deepest UIA element spanning this text.

### 3.8 Windows 11 UIA Remote Operations (`IUIAutomationRemoteOperation`)
In Windows 11 22H2+, Microsoft added UIA Remote Operations:
- Allows the screen reader to compile a bytecode instruction block (evaluating conditions, looping children, querying properties).
- Uploads the bytecode to the OS via `IUIAutomationRemoteOperation::Execute()`.
- Executes inside the target application's process without custom DLL injection, returning results in **a single RPC round-trip**.

---

## 4. MSAA & WinEvents Complete Specification

### 4.1 Global WinEvent Hook Setup
- **API Call:** `SetWinEventHook(EVENT_MIN, EVENT_MAX, NULL, winEventCallback, 0, 0, WINEVENT_OUTOFCONTEXT)`
- **Thread Model:** Runs on an STA thread with a Win32 message pump (`GetMessageW`).

### 4.2 Complete WinEvent Code Table

| WinEvent Hex ID | Constant Name | Screen Reader Meaning |
| :--- | :--- | :--- |
| `0x0003` | `EVENT_SYSTEM_FOREGROUND` | Active foreground window changed (Alt+Tab, window open). |
| `0x0004` | `EVENT_SYSTEM_MENUSTART` | Menu bar initiated. |
| `0x0005` | `EVENT_SYSTEM_MENUEND` | Menu bar closed. |
| `0x0006` | `EVENT_SYSTEM_MENUPOPUPSTART` | Context menu or drop-down opened. |
| `0x0007` | `EVENT_SYSTEM_MENUPOPUPEND` | Context menu or drop-down closed. |
| `0x0012` | `EVENT_SYSTEM_SCROLLINGSTART` | Window or control began scrolling. |
| `0x0016` | `EVENT_SYSTEM_SWITCHSTART` | Alt+Tab fast task switcher displayed. |
| `0x0017` | `EVENT_SYSTEM_SWITCHEND` | Alt+Tab fast task switcher dismissed. |
| `0x0020` | `EVENT_SYSTEM_DESKTOPSWITCH` | Desktop switched (e.g. UAC Prompt, Lock Screen). |
| `0x8002` | `EVENT_OBJECT_SHOW` | Hidden UI control became visible. |
| `0x8003` | `EVENT_OBJECT_HIDE` | Visible UI control hidden. |
| `0x8005` | `EVENT_OBJECT_FOCUS` | Keyboard focus changed to element. |
| `0x8006` | `EVENT_OBJECT_SELECTION` | Selection changed to single item. |
| `0x8007` | `EVENT_OBJECT_SELECTIONADD` | Item added to multiple selection. |
| `0x8008` | `EVENT_OBJECT_SELECTIONREMOVE`| Item deselected in multiple selection. |
| `0x800A` | `EVENT_OBJECT_STATECHANGE` | CheckBox, Toggle, or Expansion state toggled. |
| `0x800C` | `EVENT_OBJECT_NAMECHANGE` | Accessible name or label updated. |
| `0x800E` | `EVENT_OBJECT_VALUECHANGE` | Text value, slider, or progress bar changed. |

### 4.3 Converting WinEvents to `IAccessible`
In `winEventCallback(hHook, event, hwnd, idObject, idChild, idEventThread, dwmsEventTime)`:
```rust
let mut p_acc: Option<IAccessible> = None;
let mut var_child = VARIANT::default();
let hr = AccessibleObjectFromEvent(
    hwnd,
    idObject,
    idChild,
    &mut p_acc as *mut _,
    &mut var_child as *mut _,
);
```

### 4.4 Complete MSAA Role Table (`oleacc.h`)

| MSAA Role ID | Constant | Unified Role |
| :--- | :--- | :--- |
| `1` | `ROLE_SYSTEM_TITLEBAR` | `Role::TitleBar` |
| `2` | `ROLE_SYSTEM_MENUBAR` | `Role::MenuBar` |
| `3` | `ROLE_SYSTEM_SCROLLBAR` | `Role::ScrollBar` |
| `8` | `ROLE_SYSTEM_ALERT` | `Role::Alert` |
| `9` | `ROLE_SYSTEM_WINDOW` | `Role::Window` |
| `10` | `ROLE_SYSTEM_CLIENT` | `Role::Pane` |
| `11` | `ROLE_SYSTEM_MENUPOPUP` | `Role::PopupMenu` |
| `12` | `ROLE_SYSTEM_MENUITEM` | `Role::MenuItem` |
| `13` | `ROLE_SYSTEM_TOOLTIP` | `Role::ToolTip` |
| `14` | `ROLE_SYSTEM_APPLICATION` | `Role::Application` |
| `15` | `ROLE_SYSTEM_DOCUMENT` | `Role::Document` |
| `16` | `ROLE_SYSTEM_PANE` | `Role::Pane` |
| `18` | `ROLE_SYSTEM_DIALOG` | `Role::Dialog` |
| `20` | `ROLE_SYSTEM_GROUPING` | `Role::Grouping` |
| `21` | `ROLE_SYSTEM_SEPARATOR` | `Role::Separator` |
| `22` | `ROLE_SYSTEM_TOOLBAR` | `Role::ToolBar` |
| `23` | `ROLE_SYSTEM_STATUSBAR` | `Role::StatusBar` |
| `24` | `ROLE_SYSTEM_TABLE` | `Role::Table` |
| `25` | `ROLE_SYSTEM_COLUMNHEADER` | `Role::TableColumnHeader` |
| `26` | `ROLE_SYSTEM_ROWHEADER` | `Role::TableRowHeader` |
| `29` | `ROLE_SYSTEM_CELL` | `Role::TableCell` |
| `30` | `ROLE_SYSTEM_LINK` | `Role::Link` |
| `33` | `ROLE_SYSTEM_LIST` | `Role::List` |
| `34` | `ROLE_SYSTEM_LISTITEM` | `Role::ListItem` |
| `35` | `ROLE_SYSTEM_OUTLINE` | `Role::TreeView` |
| `36` | `ROLE_SYSTEM_OUTLINEITEM` | `Role::TreeViewItem` |
| `37` | `ROLE_SYSTEM_PAGETAB` | `Role::Tab` |
| `38` | `ROLE_SYSTEM_PROPERTYPAGE` | `Role::PropertyPage` |
| `40` | `ROLE_SYSTEM_GRAPHIC` | `Role::Graphic` |
| `41` | `ROLE_SYSTEM_STATICTEXT` | `Role::StaticText` |
| `42` | `ROLE_SYSTEM_TEXT` | `Role::EditableText` |
| `43` | `ROLE_SYSTEM_PUSHBUTTON` | `Role::Button` |
| `44` | `ROLE_SYSTEM_CHECKBUTTON` | `Role::CheckBox` |
| `45` | `ROLE_SYSTEM_RADIOBUTTON` | `Role::RadioButton` |
| `46` | `ROLE_SYSTEM_COMBOBOX` | `Role::ComboBox` |
| `48` | `ROLE_SYSTEM_PROGRESSBAR` | `Role::ProgressBar` |
| `49` | `ROLE_SYSTEM_SLIDER` | `Role::Slider` |
| `50` | `ROLE_SYSTEM_SPINBUTTON` | `Role::SpinButton` |
| `60` | `ROLE_SYSTEM_PAGETABLIST` | `Role::TabControl` |
| `62` | `ROLE_SYSTEM_SPLITBUTTON` | `Role::SplitButton` |

### 4.5 Complete MSAA State Bitflags (`oleacc.h`)

| Bitflag Value | Constant | Unified State Flag |
| :--- | :--- | :--- |
| `0x00000001` | `STATE_SYSTEM_UNAVAILABLE` | `State::UNAVAILABLE` |
| `0x00000002` | `STATE_SYSTEM_SELECTED` | `State::SELECTED` |
| `0x00000004` | `STATE_SYSTEM_FOCUSED` | `State::FOCUSED` |
| `0x00000008` | `STATE_SYSTEM_PRESSED` | `State::PRESSED` |
| `0x00000010` | `STATE_SYSTEM_CHECKED` | `State::CHECKED` |
| `0x00000020` | `STATE_SYSTEM_MIXED` | `State::HALFCHECKED` |
| `0x00000040` | `STATE_SYSTEM_READONLY` | `State::READONLY` |
| `0x00000080` | `STATE_SYSTEM_HOTTRACKED` | `State::HOTTRACKED` |
| `0x00000200` | `STATE_SYSTEM_EXPANDED` | `State::EXPANDED` |
| `0x00000400` | `STATE_SYSTEM_COLLAPSED` | `State::COLLAPSED` |
| `0x00000800` | `STATE_SYSTEM_BUSY` | `State::BUSY` |
| `0x00004000` | `STATE_SYSTEM_INVISIBLE` | `State::INVISIBLE` |
| `0x00008000` | `STATE_SYSTEM_OFFSCREEN` | `State::OFFSCREEN` |
| `0x00040000` | `STATE_SYSTEM_FOCUSABLE` | `State::FOCUSABLE` |
| `0x00080000` | `STATE_SYSTEM_SELECTABLE` | `State::SELECTABLE` |
| `0x00100000` | `STATE_SYSTEM_LINKED` | `State::LINKED` |
| `0x00200000` | `STATE_SYSTEM_TRAVERSED` | `State::VISITED` |
| `0x00400000` | `STATE_SYSTEM_MULTISELECTABLE` | `State::MULTISELECTABLE` |
| `0x02000000` | `STATE_SYSTEM_PROTECTED` | `State::PROTECTED` |
| `0x40000000` | `STATE_SYSTEM_HASPOPUP` | `State::HASPOPUP` |

---

## 5. Direct Win32 Window & Common Control Inspection

When controls lack proper MSAA/UIA implementations or when UIA reports stale data, `bit_sr` queries controls directly via Win32 messages.

### 5.1 Common Edit Controls (`Edit`, `RichEdit20`, `RICHEDIT50W`)
- **Messages:**
  - `EM_GETSEL (0x00B0)`: Returns `(start_offset, end_offset)` packed into `LRESULT`.
  - `EM_LINEFROMCHAR (0x00C9)`: Maps character index to line index.
  - `EM_LINEINDEX (0x00BB)`: Maps line index to character offset of line start.
  - `EM_LINELENGTH (0x00C1)`: Returns character count of line.
  - `EM_GETLINE (0x00C4)`: Populates buffer with text of specified line.
  - `WM_GETTEXTLENGTH (0x000E)` / `WM_GETTEXT (0x000D)`.

### 5.2 Common List View (`SysListView32`) Cross-Process Reading
Common Controls expect structure pointers allocated inside their own process space. Calling `LVM_GETITEMTEXTW` with a pointer from the screen reader process causes an instant memory violation (`0xC0000005`) in the target process.

#### The Out-Of-Process Protocol:
1. `h_process = OpenProcess(PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE, FALSE, pid)`.
2. Remote allocation:
   ```rust
   let remote_buf = VirtualAllocEx(
       h_process,
       None,
       size_of::<LVITEMW>() + 512,
       MEM_COMMIT,
       PAGE_READWRITE,
   );
   ```
3. Prepare local `LVITEMW` with `mask = LVIF_TEXT`, pointing `pszText` to `remote_buf + size_of::<LVITEMW>()`.
4. `WriteProcessMemory(h_process, remote_buf, &local_item, size_of::<LVITEMW>(), None)`.
5. Send message with timeout:
   ```rust
   let mut result: usize = 0;
   SendMessageTimeoutW(
       hwnd,
       LVM_GETITEMTEXTW,
       item_index,
       remote_buf as isize,
       SMTO_ABORTIFHUNG | SMTO_NORMAL,
       1000,
       Some(&mut result),
   );
   ```
6. `ReadProcessMemory(h_process, remote_buf + size_of::<LVITEMW>(), &mut local_str_buf, 512, None)`.
7. `VirtualFreeEx(h_process, remote_buf, 0, MEM_RELEASE)`.
8. `CloseHandle(h_process)`.

---

## 6. Comprehensive Windows File Explorer Reference

### 6.1 Window Hierarchy Catalog

```
[CabinetWClass] (explorer.exe)
  ├── [WorkerW] / [ReBarWindow32]
  │     ├── [Address Band Root]
  │     │     ├── [msctls_progress32] ──► CONTAINS BREADCRUMB. HIDE PROGRESS BAR ROLE!
  │     │     │     └── [Breadcrumb Parent]
  │     │     │           └── [ToolbarWindow32] (Path buttons: "This PC", "Drive C")
  │     │     └── [ComboBoxEx32] / [Edit] (Active path entry when Alt+D is pressed)
  │     └── [UniversalSearchBand] / [Search Box]
  │           └── [SearchEditBox] (Search field)
  ├── [XamlExplorerHostIslandWindow] (Windows 11 22H2+ XAML Shell Host)
  │     └── [DesktopWindowXamlSource]
  │           ├── [TabListView] (Tab control hosting TabItem elements)
  │           └── [CommandBar] (WinUI buttons: New, Cut, Copy, Paste, View)
  ├── [ShellTabWindowClass] (Content Container)
  │     └── [DUIViewWndClassName]
  │           └── [DirectUIHWND]
  │                 ├── [UIFolderTreeView] (Navigation Pane on left)
  │                 └── [UIItemsView] (File & folder list)
  │                       ├── [ListItem]: "Documents"
  │                       └── [ListItem]: "notes.txt"
  └── [StatusBarModuleInner] / [msctls_statusbar32] (Bottom status bar)
```

### 6.2 The Complete Explorer Quirks & Algorithms Catalog

#### Quirk 1: The Address Bar Progress Bar Illusion
- **Detection:** `windowClassName == "msctls_progress32"` AND ancestor class is `"Address Band Root"`.
- **Handling:** Override role to `Role::Pane` (layout only). Never announce percentage or value changes. Only process focus events when they land on `ToolbarWindow32` (breadcrumbs) or `Edit` (typed path).

#### Quirk 2: Desktop / SysListView32 Duplicate Focus Events
- **Detection:** `windowClassName == "SysListView32"` and role is `LISTITEM`.
- **Handling:** Maintain `last_focus_tuple = (hwnd, object_id, child_id)`. If an incoming focus event arrives with the exact same tuple within 60ms, drop it silently.

#### Quirk 3: WorkerW "Pane" Announcement on Minimize
- **Detection:** `windowClassName == "WorkerW"` AND `role == Role::Pane` AND `name.is_none()`.
- **Handling:** Discard `gainFocus` event immediately.

#### Quirk 4: UniversalSearchBand Redundant MSAA Focus
- **Detection:** `windowClassName in ("Search Box", "UniversalSearchBand")` AND incoming event is MSAA `EVENT_OBJECT_FOCUS`.
- **Handling:** Discard MSAA focus event because UIA's `SearchEditBox` fires the primary, correctly-labeled focus event.

#### Quirk 5: Unicode BiDi Markers in Details View
- **Detection:** `className == "UIProperty"` or any text extracted from File Explorer column cells.
- **Handling:** Apply regex/filter to remove `\u{200E}` (Left-to-Right Mark) and `\u{200F}` (Right-to-Left Mark) before sending text to speech.

#### Quirk 6: Status Bar Parsing (`StatusBarModuleInner`)
- **Structure:**
  - Child Group 1: Has 1 StaticText child = Item count (`"24 items"`).
  - Child Group 2: Has 1 StaticText child = Selection summary (`"1 item selected  14.2 KB"`). If nothing selected, Group 2 is absent.
  - Child Group 3: Radio buttons for view modes.
- **Algorithm:**
  ```rust
  let mut parts = Vec::new();
  for child in status_bar.children() {
      if child.role() == Role::Grouping {
          if let Some(text_child) = child.first_child() {
              if text_child.role() == Role::StaticText {
                  if let Some(name) = text_child.name() {
                      parts.push(name);
                  }
              }
          }
      }
  }
  let status_text = parts.join(", "); // e.g. "24 items, 1 item selected 14.2 KB"
  ```

#### Quirk 7: Windows 11 Tabs (`TabListView`) Selection Deduplication
- **Detection:** `event == UIA_SelectionItem_ElementSelectedEventId` AND `obj.role() == Role::Tab` AND `parent.automation_id() == "TabListView"`.
- **Handling:** Explorer fires 2 selection events per tab switch. Maintain `last_selected_tab_id`. If `last_selected_tab_id == current_tab_id`, drop the second event.

#### Quirk 8: Task Switcher (`Alt+Tab`) MultitaskingViewFrame
- **Detection:** `className in ("MultitaskingViewFrame", "Windows.UI.Input.InputSite.WindowClass")`.
- **Handling:** Block focus events on the container frame (`shouldAllowFocus = false`). For child `ListItem` items, if `GetAsyncKeyState(VK_MENU) & 0x8000 != 0`, set container parent directly to Desktop to suppress repeating the task switcher title.

#### Quirk 9: Systray Focus Bounce Bug
- **Detection:** Focus moves to Notification Area (`ToolbarWindow32` under `SysPager`).
- **Handling:** If the physical mouse cursor is sitting on another tray icon, Windows Explorer bounces focus back to the mouse position. If focus enters systray via keyboard, call `SetCursorPos(0, 0)` to move the mouse away.

---

## 7. Focus Tracking, Ancestry Resolution & Speech Formatting

### 7.1 The Ancestry Line Algorithm (`api.py`)
When focus changes from `OldFocus` to `NewFocus`, announcing the entire tree from the desktop down creates excessive verbosity.
1. Walk `NewFocus.parent()` upwards to build the vector:
   `[Desktop, Window, Pane, List, ListItem]`
2. Walk backwards comparing with `old_ancestors` to find the convergence point:
   ```rust
   let mut focus_difference_level = 0;
   for (index, old_anc) in old_ancestors.iter().enumerate() {
       if new_ancestors.get(index) != Some(old_anc) {
           focus_difference_level = index;
           break;
       }
   }
   ```
3. **Speech Output Generation:**
   - Only speak ancestors starting from `focus_difference_level` down to `NewFocus`.
   - If switching between items in the same list, `focus_difference_level == 4` $\to$ speak only the new `ListItem`.
   - If `Alt+Tab` into File Explorer from Chrome, `focus_difference_level == 1` $\to$ speak Window Title ("Documents - File Explorer"), then Container ("Items View"), then `ListItem`.

### 7.2 Property Speech Formatting Sequence
Properties must be formatted in strict logical order:
1. **Name:** e.g. `"Documents"`
2. **Role:** e.g. `"folder"` or `"button"` (suppressed if name already contains role)
3. **Value:** e.g. `"C:\Users\Admin"` (for edit controls, sliders)
4. **States:** e.g. `"selected"`, `"checked"`, `"expanded"`
5. **Keyboard Shortcut:** e.g. `"Ctrl+O"`
6. **Position in Set:** e.g. `"1 of 24"`
7. **Description / HelpText:** (announced only if requested or configured)

### 7.3 Instant Speech Cancellation
On any `KeyDown` in `WH_KEYBOARD_LL` that is not a modifier key, immediately call:
`speech_dispatcher.cancel_speech()`.
This gives the user the critical sub-millisecond response when scrolling with arrow keys.

---

## 8. Multi-Desktop, Security & Session Tracking

### 8.1 Windows Secure Desktop (UAC & Winlogon)
- When a UAC elevation prompt appears, Windows switches from the `"Default"` desktop to the `"Winlogon"` desktop.
- **Detection:** WinEvent `EVENT_SYSTEM_DESKTOPSWITCH (0x0020)` fires.
- **Handling:**
  ```rust
  let h_desktop = OpenInputDesktop(0, false, DESKTOP_SWITCHDESKTOP);
  if h_desktop.is_invalid() {
      // Switched to secure desktop where non-elevated apps have no read access.
      speech.cancel_speech();
      ui.message("Secure Desktop");
      enter_sleep_mode();
  }
  ```

### 8.2 Session Lock Tracking (`WTSQuerySessionInformationW`)
To protect user privacy, when Windows is locked (`Win+L`), the screen reader must lock down settings and object navigation:
- Hook `WTSRegisterSessionNotification(message_hwnd, NOTIFY_FOR_THIS_SESSION)`.
- Listen for `WM_WTSSESSION_CHANGE` with `WPARAM = WTS_SESSION_LOCK (7)` and `WTS_SESSION_UNLOCK (8)`.

### 8.3 Application Manifest & `uiAccess="true"`
To interact with elevated administrator windows (e.g. Admin Command Prompt, Task Manager) without running the screen reader itself as Administrator:
- The executable manifest must include:
  ```xml
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="true" />
      </requestedPrivileges>
    </security>
  </trustInfo>
  ```
- **Requirements enforced by Windows:**
  1. The binary must be digitally signed by a certificate in the Trusted Root Certification Authorities store.
  2. The binary must reside in a secure location: `C:\Program Files\` or `C:\Windows\System32\`.

---

## 9. Deadlock Prevention, Hung Windows & Watchdog Architecture

Screen readers make synchronous cross-process calls. If a target application freezes, a synchronous COM call or `SendMessage` will block the screen reader forever.

### 9.1 Hung Window Detection
Before sending messages or invoking COM on an HWND:
```rust
if IsHungAppWindow(hwnd).as_bool() {
    return; // Skip target completely
}
```

### 9.2 Safe Messaging Invariant
**NEVER call plain `SendMessageW`.** Always use `SendMessageTimeoutW`:
```rust
let mut result: usize = 0;
let ok = SendMessageTimeoutW(
    hwnd,
    msg,
    w_param,
    l_param,
    SMTO_ABORTIFHUNG | SMTO_NORMAL,
    500, // 500ms timeout
    Some(&mut result),
);
```

### 9.3 The Watchdog Timer
A separate watcher thread uses `CreateWaitableTimerW` and `SetWaitableTimer`. The Core thread calls `watchdog.alive()` on every event pump iteration. If the timer signals (heartbeat missing for >3 seconds), the watchdog emits an audio warning and triggers thread stack unwinding / call cancellation.

---

## 10. Display Model & Screen Scraping Fallback

For legacy applications that provide neither UIA nor MSAA (e.g. legacy terminal emulators, custom GDI engines):
- **Display Model:** Hooks Win32 GDI text rendering calls (`ExtTextOutW`, `TextOutW`, `DrawTextExW`) using Microsoft Detours / MinHook.
- **Screen Scraping:**
  - `GetDC(hwnd)`, `CreateCompatibleDC(hdc)`, `CreateCompatibleBitmap(...)`.
  - `BitBlt(mem_dc, ..., hdc, ..., SRCCOPY)`.
  - Text is extracted into a 2D spatial text coordinate grid (`displayModel.py`), allowing character-by-character review cursor navigation across pixels.

---

## 11. Master Windows API, COM Interface & Constant Reference Table

| Category | API / Interface / Constant | Value / Signature | Source Header | Rust `windows` Crate Feature |
| :--- | :--- | :--- | :--- | :--- |
| **Input Hook** | `SetWindowsHookExW` | `(idHook, lpfn, hmod, dwThreadId) -> HHOOK` | `winuser.h` | `Win32_UI_WindowsAndMessaging` |
| **Input Hook** | `CallNextHookEx` | `(hhk, nCode, wParam, lParam) -> LRESULT` | `winuser.h` | `Win32_UI_WindowsAndMessaging` |
| **Input Hook** | `UnhookWindowsHookEx`| `(hhk) -> BOOL` | `winuser.h` | `Win32_UI_WindowsAndMessaging` |
| **Input Hook** | `WH_KEYBOARD_LL` | `13` | `winuser.h` | `Win32_UI_WindowsAndMessaging` |
| **Input Hook** | `WH_MOUSE_LL` | `14` | `winuser.h` | `Win32_UI_WindowsAndMessaging` |
| **Input State**| `GetAsyncKeyState` | `(vKey: i32) -> i16` | `winuser.h` | `Win32_UI_Input_KeyboardAndMouse` |
| **Input State**| `GetKeyState` | `(vKey: i32) -> i16` | `winuser.h` | `Win32_UI_Input_KeyboardAndMouse` |
| **Input State**| `SendInput` | `(cInputs, pInputs, cbSize) -> u32` | `winuser.h` | `Win32_UI_Input_KeyboardAndMouse` |
| **Input State**| `ToUnicodeEx` | `(wVirtKey, wScanCode, lpKeyState, ...) -> i32`| `winuser.h` | `Win32_UI_Input_KeyboardAndMouse` |
| **COM Core** | `CoInitializeEx` | `(pvReserved, dwCoInit) -> HRESULT` | `objbase.h` | `Win32_System_Com` |
| **COM Core** | `COINIT_MULTITHREADED`| `0x0` | `objbase.h` | `Win32_System_Com` |
| **COM Core** | `COINIT_APARTMENTTHREADED`| `0x2` | `objbase.h` | `Win32_System_Com` |
| **COM Core** | `CoCreateInstance` | `(rclsid, pUnkOuter, dwClsContext, riid, ppv)`| `objbase.h` | `Win32_System_Com` |
| **UIA Core** | `CLSID_CUIAutomation8` | `e22ad333-b25f-460c-83d0-0581107395c9` | `uiautomationclient.h` | `Win32_UI_Accessibility` |
| **UIA Core** | `IUIAutomation6` | Interface GUID: `c32f6f12-1e1f-4145-b40b-32e203473b6e` | `uiautomationclient.h` | `Win32_UI_Accessibility` |
| **UIA Core** | `UiaHasServerSideProvider`| `(hwnd: HWND) -> BOOL` | `uiautomationcoreapi.h` | `Win32_UI_Accessibility` |
| **MSAA Core** | `SetWinEventHook` | `(eventMin, eventMax, hmodWinEventProc, ...) -> HWINEVENTHOOK` | `winuser.h` | `Win32_UI_Accessibility` |
| **MSAA Core** | `AccessibleObjectFromEvent`| `(hwnd, dwId, dwChildId, ppacc, pvarChild) -> HRESULT` | `oleacc.h` | `Win32_UI_Accessibility` |
| **MSAA Core** | `AccessibleChildren` | `(paccContainer, iChildStart, cChildren, ...) -> HRESULT` | `oleacc.h` | `Win32_UI_Accessibility` |
| **Windowing** | `GetForegroundWindow`| `() -> HWND` | `winuser.h` | `Win32_UI_WindowsAndMessaging` |
| **Windowing** | `GetGUIThreadInfo` | `(idThread, pgui) -> BOOL` | `winuser.h` | `Win32_UI_WindowsAndMessaging` |
| **Windowing** | `IsHungAppWindow` | `(hwnd: HWND) -> BOOL` | `winuser.h` | `Win32_UI_WindowsAndMessaging` |
| **Windowing** | `SendMessageTimeoutW`| `(hWnd, Msg, wParam, lParam, fuFlags, uTimeout, lpdwResult) -> LRESULT` | `winuser.h` | `Win32_UI_WindowsAndMessaging` |
| **Memory** | `VirtualAllocEx` | `(hProcess, lpAddress, dwSize, flAllocationType, flProtect) -> LPVOID` | `memoryapi.h` | `Win32_System_Memory` |
| **Memory** | `ReadProcessMemory` | `(hProcess, lpBaseAddress, lpBuffer, nSize, lpNumberOfBytesRead) -> BOOL`| `memoryapi.h` | `Win32_System_Diagnostics_Debug` |
| **Memory** | `WriteProcessMemory`| `(hProcess, lpBaseAddress, lpBuffer, nSize, lpNumberOfBytesWritten) -> BOOL`| `memoryapi.h` | `Win32_System_Diagnostics_Debug` |
| **Desktop** | `OpenInputDesktop` | `(dwFlags, fInherit, dwDesiredAccess) -> HDESK` | `winuser.h` | `Win32_System_StationsAndDesktops` |
| **Desktop** | `SetThreadDesktop` | `(hDesktop: HDESK) -> BOOL` | `winuser.h` | `Win32_System_StationsAndDesktops` |
| **Session** | `WTSRegisterSessionNotification`| `(hWnd: HWND, dwFlags: DWORD) -> BOOL` | `wtsapi32.h` | `Win32_System_RemoteDesktop` |

---

## 12. Rust Implementation Blueprint for `bit_sr_platform_windows`

### Crate Structure
```text
crates/bit_sr_platform_windows/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Public traits & platform lifecycle
    ├── error.rs                # Windows COM / Win32 HRESULT error types
    ├── com.rs                  # MTA Apartment initialization wrapper
    ├── uia/
    │   ├── mod.rs              # IUIAutomation6 client wrapper & connection recovery
    │   ├── cache.rs            # IUIAutomationCacheRequest builder with 18 core properties
    │   ├── events.rs           # COM event handler callbacks (Focus, Property, Notification)
    │   ├── element.rs          # Safe wrapper around IUIAutomationElement (reads cached props)
    │   ├── tree.rs             # ControlViewWalker and RawViewWalker navigation
    │   └── patterns.rs         # Value, SelectionItem, TextPattern2 wrappers
    ├── msaa/
    │   ├── mod.rs              # SetWinEventHook listener thread
    │   ├── accessible.rs       # Safe wrapper around IAccessible & childIDs
    │   └── roles_states.rs     # MSAA to unified Role/State bitflags conversion
    ├── input/
    │   ├── mod.rs              # Dedicated input hook thread manager
    │   ├── keyboard.rs         # Low-level keyboard hook callback (WH_KEYBOARD_LL)
    │   └── mouse.rs            # Low-level mouse hook callback (WH_MOUSE_LL)
    ├── apps/
    │   ├── mod.rs              # App module dispatcher
    │   └── explorer.rs         # Windows File Explorer special-casing (all 9 quirks implemented)
    ├── common_controls/
    │   ├── mod.rs              # Direct Win32 common controls message queries
    │   ├── listview.rs         # SysListView32 out-of-process memory reader
    │   └── edit.rs             # Edit / RichEdit caret and selection queries
    ├── desktop.rs              # Secure desktop detection & thread desktop switching
    └── watchdog.rs             # Waitable timer thread guarding against hung windows
```

### Cargo Dependencies (`Cargo.toml`)
```toml
[package]
name = "bit_sr_platform_windows"
version = "0.1.0"
edition = "2024"

[dependencies]
bit_sr_core = { path = "../bit_sr_core" }
crossbeam-channel = "0.5"
bitflags = "2.4"
regex = "1.10"
log = "0.4"

[dependencies.windows]
version = "0.60"
features = [
    "Win32_Foundation",
    "Win32_System_Com",
    "Win32_System_Threading",
    "Win32_System_Memory",
    "Win32_System_StationsAndDesktops",
    "Win32_System_RemoteDesktop",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Accessibility",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_Graphics_Gdi",
    "Win32_Media_Audio",
]
```
