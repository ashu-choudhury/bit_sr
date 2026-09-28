# Windows Subsystem & API Integration Specification: `bit_sr_platform_windows`

> **Document Status:** Reference & Architecture Specification  
> **Source Analysis:** NVDA Source Codebase (`references/nvda`), Windows Accessibility Architecture, and `explorer.exe` Internal Mechanics.  
> **Target:** Native Rust Implementation for Windows (`bit_sr_platform_windows`).

---

## 1. Architectural Overview: How Screen Readers Interface with Windows

Windows does not have a single unified accessibility pipeline. Instead, assistive technologies must navigate **three distinct generations of Windows APIs**, coupled with low-level kernel/user hooks:

```
                               ┌────────────────────────────────────────────────────────┐
                               │                    bit_sr Core                         │
                               └───────────────────────────▲────────────────────────────┘
                                                           │
                               ┌───────────────────────────┴────────────────────────────┐
                               │               bit_sr_platform_windows                  │
                               └───────┬───────────────────┬────────────────────┬───────┘
                                       │                   │                    │
              ┌────────────────────────▼─────┐ ┌───────────▼───────────┐ ┌──────▼────────────────────┐
              │      UIA Subsystem (MTA)     │ │   MSAA / WinEvents    │ │    Low-Level Input Hooks   │
              │  - IUIAutomation (3..6)      │ │   - SetWinEventHook   │ │    - WH_KEYBOARD_LL        │
              │  - CacheRequests             │ │   - AccessibleObject- │ │    - WH_MOUSE_LL           │
              │  - Focus/Property Handlers   │ │     FromEvent         │ │    - SendInput             │
              │  - UiaHasServerSideProvider  │ │   - IAccessible       │ │    - GetAsyncKeyState      │
              └──────────────┬───────────────┘ └───────────┬───────────┘ └──────────────┬─────────────┘
                             │                             │                            │
 ┌───────────────────────────▼─────────────────────────────▼────────────────────────────▼─────────────┐
 │                                   Target Application / OS Process                                  │
 │   File Explorer (explorer.exe), Desktop, XAML Islands, Classic Win32 Controls, Modern WinUI 3      │
 └────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### The Three API Layers

1. **Microsoft UI Automation (UIA) [Modern]:**
   - The COM-based accessibility framework introduced in Windows Vista and substantially enhanced in Windows 8.1, 10, and 11.
   - Operates primarily out-of-process via COM RPC.
   - Required for Windows 10/11 Shell, XAML Islands, Windows Terminal, Modern File Explorer (`CabinetWClass`), Edge, and modern WinUI 3 applications.
2. **Microsoft Active Accessibility (MSAA / IAccessible) [Legacy]:**
   - The original Win32 accessibility model introduced in Windows 95/98.
   - Communicates via `oleacc.dll` and `WinEvents`.
   - Still required for classic common controls (`SysListView32`, `SysTreeView32`, standard menus `#32768`, legacy dialogs) where Microsoft's built-in UIA-to-MSAA proxy has severe bugs, lag, or dropped events.
3. **Win32 System Hooks & Windowing (`user32.dll` / `kernel32.dll`):**
   - Global low-level keyboard hooks (`WH_KEYBOARD_LL`) to intercept keystrokes before applications see them (essential for hotkeys, modifier key handling, and immediate speech cancellation on keypress).
   - Window message inspection (`SendMessageTimeoutW`, `GetGUIThreadInfo`, `GetForegroundWindow`).

---

## 2. Windows Hooks, Message Loops & System Subsystems in NVDA

### 2.1 Low-Level Keyboard & Mouse Hooks (`winInputHook.py`)
NVDA intercepts user input at the kernel/user32 boundary:

- **Hook Function:** `SetWindowsHookExW(WH_KEYBOARD_LL, keyboardHookProc, hModule, 0)`
- **Mouse Hook:** `SetWindowsHookExW(WH_MOUSE_LL, mouseHookProc, hModule, 0)`
- **Crucial Rule:** The low-level hook **must run on a dedicated OS thread** running a standard Win32 message pump:
  ```rust
  let mut msg = MSG::default();
  while GetMessageW(&mut msg, None, 0, 0).as_bool() {
      // Loop pumps hook callbacks
  }
  ```
- **How Keys are Handled:**
  - Key down events inspect `KBDLLHOOKSTRUCT` (`vkCode`, `scanCode`, `flags`).
  - If a key matches a screen reader command (e.g. `NVDA/Caps+Down`, or speech interruption on any key), the hook returns `1` (consumes the key, preventing the focused application from receiving it).
  - If it is standard navigation (e.g. `Tab`, `Arrow Up/Down` in File Explorer), the hook passes the key through via `CallNextHookEx` so the application can change focus naturally.
- **Latency Deadline:** Windows enforces a hook timeout (default ~200-300ms, defined in registry `LowLevelHooksTimeout`). If the hook procedure blocks, Windows silently unhooks it! Therefore, the callback must never perform heavy work or synchronous COM calls; it must immediately delegate to a lock-free queue and return.

### 2.2 UI Automation Subsystem (`UIAHandler/__init__.py`)
NVDA's UIA implementation reveals critical architectural rules:

1. **Dedicated MTA Thread:**
   - UI Automation COM calls **must** be executed on a thread initialized with `COINIT_MULTITHREADED` (`CoInitializeEx(NULL, COINIT_MULTITHREADED)`).
   - Running UIA on an STA (Single-Threaded Apartment) thread causes COM cross-thread marshaling deadlocks with window message loops.
2. **Interface Version Selection:**
   - NVDA instantiates `CLSID_CUIAutomation8` and queries the highest supported interface: `IUIAutomation6` (Win 10 1809+) down to `IUIAutomation3` (Win 8.1).
3. **Disabling MSAA-to-UIA Proxy WinEvents Mapping:**
   - By default, UIA attempts to listen to legacy MSAA events and convert them into UIA property change events. This causes severe unresponsiveness when legacy applications have slow message pumps.
   - NVDA walks `IUIAutomation::ProxyFactoryMapping` and clears all WinEvents mapped to UIA events for `UIA_AutomationPropertyChangedEventId`.
4. **Performance Features (`IUIAutomation6`):**
   - `CoalesceEvents = CoalesceEventsOptions_Enabled`: Instructs Windows to merge duplicate/burst events before sending them over RPC.
   - `ConnectionRecoveryBehavior = ConnectionRecoveryBehaviorOptions_Enabled`: Automatically restores broken UIA connections if the provider crashed.
5. **The `CacheRequest` Strategy:**
   - Calling property getters (`get_CurrentName`, `get_CurrentControlType`) over COM causes a synchronous cross-process RPC roundtrip (~0.5ms - 5ms each). Querying 10 properties on an element would cause unacceptable stutter.
   - **Solution:** A `IUIAutomationCacheRequest` is attached to every event handler registration and tree walker. The operating system pre-populates these properties into the element before dispatching the event:
     - `UIA_FrameworkIdPropertyId`
     - `UIA_AutomationIdPropertyId`
     - `UIA_ClassNamePropertyId`
     - `UIA_ControlTypePropertyId`
     - `UIA_NamePropertyId`
     - `UIA_LocalizedControlTypePropertyId`
     - `UIA_HasKeyboardFocusPropertyId`
     - `UIA_IsContentElementPropertyId`
     - `UIA_IsControlElementPropertyId`
     - `UIA_ProcessIdPropertyId`
     - Pattern: `UIA_TextPatternId`, `UIA_SelectionItemPatternId`
6. **Registered UIA Event Handlers:**
   - `AddFocusChangedEventHandler(cacheRequest, handler)`
   - `AddPropertyChangedEventHandler(TreeScope_Subtree, cacheRequest, handler, propertyIds)`
   - `IUIAutomationNotificationEventHandler`: (Win 10 1709+) Listens for accessibility toast/notification events (`UIA_NotificationEventId`).
   - `IUIAutomationActiveTextPositionChangedEventHandler`: Listens for caret movements in modern text controls.

### 2.3 WinEvents & MSAA Subsystem (`IAccessibleHandler/internalWinEventHandler.py`)
For windows where UIA is disabled or defective, MSAA events are hooked globally:

- **Hook Registration:**
  ```cpp
  HWINEVENTHOOK hHook = SetWinEventHook(
      EVENT_MIN, EVENT_MAX,
      NULL,
      winEventCallback,
      0, 0,
      WINEVENT_OUTOFCONTEXT
  );
  ```
- **Key WinEvents Captured:**
  - `EVENT_OBJECT_FOCUS` (0x8005): Object gained keyboard focus.
  - `EVENT_SYSTEM_FOREGROUND` (0x0003): Foreground window changed.
  - `EVENT_OBJECT_SELECTION` (0x8006): Single selection changed.
  - `EVENT_OBJECT_SELECTIONADD` (0x8007), `EVENT_OBJECT_SELECTIONREMOVE` (0x8008).
  - `EVENT_OBJECT_NAMECHANGE` (0x800C), `EVENT_OBJECT_VALUECHANGE` (0x800E), `EVENT_OBJECT_STATECHANGE` (0x800A).
  - `EVENT_OBJECT_SHOW` (0x8002), `EVENT_OBJECT_HIDE` (0x8003).
- **Retrieving the Object:**
  - When a WinEvent fires, NVDA calls `AccessibleObjectFromEvent(hwnd, objectID, childID, &pAccessible, &varChild)` from `oleacc.dll`.
  - Properties extracted: `accName`, `accRole`, `accState`, `accValue`, `accLocation`.

### 2.4 Determining UIA vs MSAA Support (`UiaHasServerSideProvider`)
How does NVDA decide whether to treat a window as a native UIA window or fall back to MSAA?
- **API Call:** `UiaHasServerSideProvider(HWND)` (exported by `UIAutomationCore.dll`).
  - Returns `TRUE` if the application natively implemented UIA server providers.
  - Returns `FALSE` if the window is an old Win32 control relying on the operating system proxy.
- **Watchdog Protection:** In NVDA, `UiaHasServerSideProvider` is wrapped in a cancellable watchdog thread because if the target application is hung, this call blocks synchronously indefinitely!

### 2.5 In-Process Injection & Helper DLLs (`nvdaHelper`)
NVDA compiles C++ helper DLLs:
- **`nvdaHelperRemote.dll`**: Injected into target processes via `SetWindowsHookEx` or `CreateRemoteThread`. Used for in-process traversal of virtual buffers (DOM nodes in Gecko/WebKit) and hooking GDI text calls (`ExtTextOutW`, `DrawTextExW`) using MinHook / Detours.
- **`UIARemote.dll` (Windows 11 Remote Operations):**
  - Uses Windows 11 `IUIAutomationRemoteOperation` API.
  - Instead of injecting a DLL or making 100 out-of-process COM calls, NVDA uploads a bytecode instruction program to the OS, which executes inside the target process and returns only the final computed result in a single transaction.

### 2.6 Multi-Desktop & Secure Desktop (UAC / Lock Screen)
- Standard Windows runs on the desktop named `"Default"`.
- UAC consent dialogs run on `"Winlogon"` or a dedicated secure desktop.
- Screen readers must call:
  - `OpenInputDesktop(0, FALSE, DESKTOP_SWITCHDESKTOP)` to detect when the desktop switches to UAC.
  - `SetThreadDesktop(hDesktop)` to switch the screen reader thread's input context to the secure desktop.
- **Manifest Requirement:** To interact with elevated windows and the secure desktop, the screen reader binary must have `uiAccess="true"` in its application manifest (`requestedExecutionLevel level="asInvoker" uiAccess="true"`), be digitally signed with a trusted certificate, and run from `Program Files`.

---

## 3. Deep Dive: Windows File Explorer (`explorer.exe`) Integration

Windows File Explorer is the foundational test case for any Windows screen reader. Over 30 years of Windows versions, File Explorer has accumulated layers of classic Win32, DirectUI, and modern WinUI 3 / XAML controls.

### 3.1 File Explorer Window & Control Anatomy

```
Top-Level Window: CabinetWClass (explorer.exe)
├── WorkerW / ReBarWindow32
│   ├── Address Band Root
│   │   ├── msctls_progress32 (Wrapper around address bar — DO NOT treat as progress bar!)
│   │   │   └── Breadcrumb Parent
│   │   │       └── ToolbarWindow32 (Breadcrumb path buttons)
│   │   └── ComboBoxEx32 / Edit (Active address input when Alt+D is pressed)
│   └── UniversalSearchBand / Search Box
│       └── SearchEditBox (Ctrl+E / F3 Search Field)
├── XamlExplorerHostIslandWindow (Win 11 22H2+) / Ribbon (Win 10)
│   └── DesktopWindowXamlSource
│       ├── TabListView (Tab strip: TabItem controls)
│       └── CommandBar (New, Cut, Copy, Paste, Share buttons)
├── ShellTabWindowClass (Main Content Host)
│   └── DUIViewWndClassName
│       └── DirectUIHWND
│           ├── UIFolderTreeView / SysTreeView32 (Left Navigation Pane: Quick Access, This PC, Drives)
│           └── UIItemsView / SysListView32 (Central File & Folder List)
│               ├── ListItem: "Documents" (Folder)
│               ├── ListItem: "project.zip" (Zip archive)
│               └── ListItem: "budget.xlsx" (Spreadsheet)
└── StatusBarModuleInner / msctls_statusbar32 (Bottom Status Bar)
    ├── Grouping: "24 items"
    └── Grouping: "1 item selected  14.2 KB"
```

### 3.2 The Specific Explorer Workarounds & Heuristics in NVDA (`source/appModules/explorer.py`)

NVDA contains extensive special-case handling for `explorer.exe`. Below is the complete catalog of behaviors and quirks:

#### 1. The Address Bar "Progress Bar" Illusion
- **The Issue:** In File Explorer, the address bar is physically embedded inside a `msctls_progress32` window whose ancestor class is `Address Band Root`. When an address changes or a folder loads, this progress bar sends state and value events.
- **The Fix:** If `windowClassName == "msctls_progress32"` and its ancestor is `Address Band Root`, force `presentationType = Layout` (hide the progress bar role completely). Only report the inner breadcrumb toolbar or edit control.

#### 2. Duplicate Focus Events on File Lists (`SysListView32`)
- **The Issue:** In desktop views or classic list views (`SysListView32`), when focus moves to a list item, Windows fires a focus event on the list control itself, immediately followed by a focus event on the focused list item.
- **The Fix:** Maintain a `lastQueuedFocusObject` cache. If a focus event matches `(windowHandle, objectID, childID)` of the previous event within 50ms, discard the duplicate event.

#### 3. The `WorkerW` Ghost Pane on Desktop / Minimize
- **The Issue:** When minimizing windows (`Win+M`) or switching to the desktop, the Explorer window `WorkerW` thread fires an MSAA `EVENT_OBJECT_FOCUS` on a window of class `WorkerW` with role `Pane` and `name == NULL`.
- **The Fix:** Silently drop `gainFocus` events for `WorkerW` when `role == PANE` and `name is None`. Otherwise, the screen reader loudly speaks "Pane" into the user's headphones every time a window minimizes.

#### 4. Search Box Redundancy (`UniversalSearchBand`)
- **The Issue:** When focusing the search box, File Explorer fires a UIA focus event on the `SearchEditBox`, immediately followed by a redundant MSAA focus event on `UniversalSearchBand` / `Search Box` with role `PANE` (#20021).
- **The Fix:** Explicitly suppress `shouldAllowIAccessibleFocusEvent` on the search band pane so only the UIA edit field speaks.

#### 5. Unicode Directionality Characters in File Properties (`UIProperty`)
- **The Issue:** In File Explorer's Details view, columns (such as "Date modified") and read-only property fields contain invisible Unicode Left-to-Right Marks (`\u200e`) and Right-to-Left Marks (`\u200f`).
- **The Fix:** Strip `\u200e` and `\u200f` from all `value` and `windowText` strings before sending them to the speech synthesizer. Otherwise, text-to-speech synthesizers stutter, pause, or mispronounce dates.

#### 6. Multi-Tasking View Frame (Alt+Tab & Task View)
- **The Issue:** In Windows 10/11, pressing `Alt+Tab` summons `MultitaskingViewFrame` or `Windows.UI.Input.InputSite.WindowClass`. It fires rapid, spurious focus events on invisible container frames before landing on the actual application icon.
- **The Fix:** Set `shouldAllowUIAFocusEvent = False` for `MultitaskingViewFrameWindow`. For the list items inside `SwitchItemListControl`, suppress focus ancestry announcements while `VK_MENU` (Alt) is held down.

#### 7. Windows 11 Tabs Navigation (`TabListView`)
- **The Issue:** In Windows 11 22H2+, File Explorer introduced tabs. Switching tabs via `Ctrl+Tab` or mouse click fires two `UIA_SelectionItem_ElementSelectedEventId` events for the exact same tab item.
- **The Fix:** When `obj.role == TAB` and `parent.automationId == "TabListView"`, check if another `UIA_elementSelected` event is already pending in the queue before speaking. Deduplicate to speak the tab title only once.

#### 8. Parsing the Status Bar (`StatusBarModuleInner`)
- **The Issue:** The File Explorer status bar is not a simple string. Under UIA (`StatusBarModuleInner`), it consists of nested grouping controls:
  - First child group: A static text child containing the total item count (e.g. `"24 items"`).
  - Second child group (optional): A static text child containing the selection count and total size (e.g. `"1 item selected  14.2 KB"`).
  - Third child group: Radio buttons for view modes (Details vs Large Icons).
- **The Fix:** Implement an explorer-specific status bar walker that iterates child groups, extracts static text elements, skips the view mode radio buttons, and formats the output cleanly as: `"24 items, 1 item selected 14.2 KB"`.

#### 9. Windows 11 Shell XAML Islands Classification
- **The Issue:** Windows 11 shell UI elements (Taskbar, system tray overflow, language switcher) host XAML islands inside `DesktopWindowXamlSource` windows. Without special handling, the operating system attempts to report them via MSAA, which fails or crashes.
- **The Fix:** Classify top-level ancestors matching `Shell_TrayWnd`, `Shell_InputSwitchTopLevelWindow`, `XamlExplorerHostIslandWindow`, and `TopLevelWindowForOverflowXamlIsland` as native UIA windows.

---

## 4. Comprehensive Windows API & System Call Catalog

The table below catalogs every Win32 API function, COM interface, and constant utilized for Windows screen reader integration, along with its Rust representation in the `windows` crate:

| Subsystem | Windows API / COM Symbol | Rust `windows` Crate Feature | Purpose in `bit_sr` |
| :--- | :--- | :--- | :--- |
| **Input Hook** | `SetWindowsHookExW` | `Win32_UI_WindowsAndMessaging` | Installs global `WH_KEYBOARD_LL` and `WH_MOUSE_LL` hooks. |
| **Input Hook** | `UnhookWindowsHookEx` | `Win32_UI_WindowsAndMessaging` | Uninstalls low-level input hooks on shutdown. |
| **Input Hook** | `CallNextHookEx` | `Win32_UI_WindowsAndMessaging` | Passes unhandled keystrokes downstream to Windows. |
| **Input Hook** | `KBDLLHOOKSTRUCT` | `Win32_UI_WindowsAndMessaging` | Memory layout for intercepted key code, scan code, and flags. |
| **Input State** | `GetAsyncKeyState`, `GetKeyState` | `Win32_UI_Input_KeyboardAndMouse` | Queries physical key press state (e.g. Alt, Ctrl, Shift, CapsLock). |
| **Input State** | `SendInput` | `Win32_UI_Input_KeyboardAndMouse` | Synthesizes keyboard/mouse events without triggering hook loops. |
| **Input State** | `ToUnicodeEx`, `GetKeyboardLayout`| `Win32_UI_Input_KeyboardAndMouse` | Translates virtual keys to typed UTF-16 characters based on active layout. |
| **Message Pump**| `GetMessageW`, `PeekMessageW` | `Win32_UI_WindowsAndMessaging` | Core message loop for hook threads. |
| **Message Pump**| `PostThreadMessageW` | `Win32_UI_WindowsAndMessaging` | Signals `WM_QUIT` to terminate dedicated background hook threads. |
| **Message Pump**| `MsgWaitForMultipleObjectsEx` | `Win32_System_Threading` | Waits for thread synchronization handles while pumping COM messages. |
| **COM Core** | `CoInitializeEx` | `Win32_System_Com` | Initializes COM apartment (`COINIT_MULTITHREADED` for UIA). |
| **COM Core** | `CoCreateInstance` | `Win32_System_Com` | Instantiates `CUIAutomation8` (`CLSID_CUIAutomation8`). |
| **UIA Core** | `IUIAutomation6` .. `IUIAutomation` | `Win32_UI_Accessibility` | Master interface for tree traversal, cache requests, event listening. |
| **UIA Core** | `IUIAutomationElement` | `Win32_UI_Accessibility` | Represents an individual accessible node in the UI hierarchy. |
| **UIA Core** | `IUIAutomationCacheRequest` | `Win32_UI_Accessibility` | Defines property batching to eliminate RPC overhead. |
| **UIA Core** | `IUIAutomationTreeWalker` | `Win32_UI_Accessibility` | Navigates parent, child, and sibling relationships across UIA trees. |
| **UIA Events**| `IUIAutomationFocusChangedEventHandler` | `Win32_UI_Accessibility` | Intercepts focus transitions across all running applications. |
| **UIA Events**| `IUIAutomationPropertyChangedEventHandler` | `Win32_UI_Accessibility` | Intercepts name, value, state, and selection property changes. |
| **UIA Events**| `IUIAutomationNotificationEventHandler` | `Win32_UI_Accessibility` | Intercepts Windows Shell notifications, toast alerts, snap layout hints. |
| **UIA Detection**| `UiaHasServerSideProvider` | `Win32_UI_Accessibility` | Determines if an HWND has native UIA support without proxying. |
| **MSAA Core** | `SetWinEventHook`, `UnhookWinEvent` | `Win32_UI_Accessibility` | Registers global WinEvent callback for legacy MSAA events. |
| **MSAA Core** | `AccessibleObjectFromEvent` | `Win32_UI_Accessibility` | Retrieves `IAccessible` interface from an incoming WinEvent. |
| **MSAA Core** | `AccessibleChildren` | `Win32_UI_Accessibility` | Iterates child elements of a legacy Win32 control. |
| **MSAA Core** | `IAccessible` | `Win32_UI_Accessibility` | Queries `accName`, `accRole`, `accState`, `accValue`. |
| **Windowing** | `GetForegroundWindow` | `Win32_UI_WindowsAndMessaging` | Identifies the currently active top-level application window. |
| **Windowing** | `GetGUIThreadInfo` | `Win32_UI_WindowsAndMessaging` | Queries caret coordinates, active window, and focus window per thread. |
| **Windowing** | `GetClassNameW` | `Win32_UI_WindowsAndMessaging` | Fetches window class (e.g. `CabinetWClass`, `SysListView32`). |
| **Windowing** | `GetWindowThreadProcessId`| `Win32_UI_WindowsAndMessaging` | Maps HWND to Process ID (`PID`) and Thread ID (`TID`). |
| **Windowing** | `IsHungAppWindow` | `Win32_UI_WindowsAndMessaging` | Detects hung/unresponsive processes before initiating blocking calls. |
| **Windowing** | `SendMessageTimeoutW` | `Win32_UI_WindowsAndMessaging` | Sends window messages with timeout (`SMTO_ABORTIFHUNG`) to prevent hang. |
| **Multi-Desktop**| `OpenInputDesktop`, `SetThreadDesktop`| `Win32_System_StationsAndDesktops` | Enables interaction with UAC prompts on Windows Secure Desktop. |
| **Sessions** | `WTSRegisterSessionNotification` | `Win32_System_RemoteDesktop` | Listens for workstation lock/unlock (`WTS_SESSION_LOCK`). |
| **Audio Ducking**| `IAudioSessionManager2` | `Win32_Media_Audio` | Lowers background application volume while screen reader is speaking. |

---

## 5. Design for the `bit_sr_platform_windows` Crate

Based on the above analysis, the Rust `bit_sr_platform_windows` crate will be organized into the following modular structure:

```text
crates/bit_sr_platform_windows/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Public traits & platform initialization
    ├── com.rs                  # MTA COM thread & initialization utilities
    ├── uia/
    │   ├── mod.rs              # UIA Master Client & event registration
    │   ├── cache.rs            # IUIAutomationCacheRequest property builder
    │   ├── events.rs           # FocusChanged, PropertyChanged event handlers
    │   ├── element.rs          # Safe wrapper around IUIAutomationElement
    │   └── patterns.rs         # Value, SelectionItem, Text, Invoke patterns
    ├── msaa/
    │   ├── mod.rs              # WinEvent hook (SetWinEventHook) & fallback dispatcher
    │   └── accessible.rs       # Safe wrapper around IAccessible & childIDs
    ├── input/
    │   ├── mod.rs              # Dedicated input hook thread
    │   ├── keyboard.rs         # WH_KEYBOARD_LL callback & key translation
    │   └── mouse.rs            # WH_MOUSE_LL callback (cursor tracking)
    ├── apps/
    │   ├── mod.rs              # App module dispatcher (by process/class)
    │   └── explorer.rs         # File Explorer (CabinetWClass) special-casing
    ├── watchdog.rs             # Timeout-guarded execution for cross-process calls
    └── desktop.rs              # Secure desktop & UAC detection (OpenInputDesktop)
```

### Key Architectural Tenets for the Rust Implementation:

1. **Zero COM Calls on the Input Hook Thread:**
   The `WH_KEYBOARD_LL` thread must only push events into a crossbeam/tokio channel and return immediately.
2. **Dedicated MTA Worker for UI Automation:**
   All UIA event subscriptions and tree walkers live inside a dedicated OS thread running `COINIT_MULTITHREADED`.
3. **Mandatory Cache Requests:**
   Never query an element's name or role individually after receiving a focus event. Always configure `IUIAutomationCacheRequest` with the 12 core properties so the operating system packages the data into the event payload.
4. **Watchdog-Guarded Invocations:**
   Every call to `UiaHasServerSideProvider` or cross-process `SendMessageTimeoutW` must be guarded so a frozen application cannot stall `bit_sr`.
5. **Explorer-First Quirk Handler:**
   The `explorer.rs` app module must be active from Day 1 to filter out the progress bar wrappers, strip Unicode BiDi markers, and deduplicate list view events.
