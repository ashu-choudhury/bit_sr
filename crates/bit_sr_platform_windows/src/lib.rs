//! Native Windows Accessibility and Input Platform Implementation.
//! 100% aligned with WINDOWS.md specification.

pub mod apps;
pub mod com;
pub mod common_controls;
pub mod desktop;
pub mod error;
pub mod input;
pub mod menu;
pub mod msaa;
pub mod text;
pub mod uia;
pub mod watchdog;

pub use apps::ExplorerFilter;
pub use com::ComGuard;
pub use common_controls::{EditControlReader, SysListView32Reader};
pub use desktop::{get_foreground_window_title, get_user_default_locale_name, is_secure_desktop_active};
pub use error::{Error, Result};
pub use input::{
    get_current_modifiers, is_browse_mode_active, is_input_help_active, set_browse_mode_active,
    set_input_help_active, KeyboardHookHandle,
};
pub use menu::{open_menu_async, show_native_popup_menu};
pub use msaa::{MsaaElement, WinEventHookHandle};
pub use text::WindowsTextProvider;
pub use uia::{create_base_cache_request, Patterns, TreeNavigator, UiaClient, UiaElement};
pub use watchdog::{is_window_hung, safe_send_message_timeout};

use bit_sr_core::events::AccessibilityEvent;
use crossbeam_channel::Sender;
use std::sync::Arc;

/// Master platform coordinator for Windows.
/// Manages the low-level keyboard hook thread, WinEvent MSAA hook thread,
/// and the dedicated UI Automation MTA worker thread.
pub struct WindowsPlatform {
    com_guard: Option<com::ComGuard>,
    keyboard_hook: Option<KeyboardHookHandle>,
    msaa_hook: Option<WinEventHookHandle>,
    uia_client: Option<Arc<UiaClient>>,
    explorer_filter: Arc<ExplorerFilter>,
}

impl WindowsPlatform {
    /// Initializes all Windows subsystems and begins streaming accessibility events.
    pub fn start(tx: Sender<AccessibilityEvent>) -> Result<Self> {
        let explorer_filter = Arc::new(ExplorerFilter::new());

        // 1. Initialize MTA COM on the current/worker thread
        log::info!("Initializing COM MTA...");
        let com_guard = com::ComGuard::init_mta().ok();

        // 2. Start low-level keyboard hook on dedicated thread
        log::info!("Starting Windows low-level keyboard hook...");
        let keyboard_hook = KeyboardHookHandle::start(tx.clone())?;

        // 3. Start MSAA / WinEvents hook on dedicated thread
        log::info!("Starting Windows MSAA WinEvent hook...");
        let msaa_hook = WinEventHookHandle::start(tx.clone())?;

        // 4. Initialize Microsoft UI Automation client
        log::info!("Starting Microsoft UI Automation client...");
        let uia_client = Arc::new(UiaClient::new(tx, explorer_filter.clone())?);

        // If foreground window is Chromium/Edge/WebView2, elevate its AXMode immediately
        unsafe {
            let fg = windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
            if !fg.0.is_null() && apps::ChromiumFilter::is_chromium_hwnd(fg) {
                log::info!("Foreground window is Chromium/Edge; activating AXMode...");
                apps::ChromiumFilter::activate_chromium_ax_mode(fg);
            }
        }

        log::info!("Windows accessibility platform successfully started!");

        Ok(Self {
            com_guard,
            keyboard_hook: Some(keyboard_hook),
            msaa_hook: Some(msaa_hook),
            uia_client: Some(uia_client),
            explorer_filter,
        })
    }

    /// Access the File Explorer heuristics and quirks filter.
    pub fn explorer_filter(&self) -> &Arc<ExplorerFilter> {
        &self.explorer_filter
    }

    /// Access the active UI Automation client.
    pub fn uia(&self) -> Option<&Arc<UiaClient>> {
        self.uia_client.as_ref()
    }

    /// Obtains a TextProvider implementation for reading text at the caret in edit controls.
    pub fn text_provider(&self) -> Option<Arc<dyn bit_sr_core::TextProvider>> {
        self.uia_client.as_ref().map(|uia| {
            Arc::new(WindowsTextProvider::new(uia.clone())) as Arc<dyn bit_sr_core::TextProvider>
        })
    }

    /// Obtains a TreeProvider implementation for harvesting accessible subtrees in web documents.
    pub fn tree_provider(&self) -> Option<Arc<dyn bit_sr_core::tree::TreeProvider>> {
        self.uia_client.as_ref().map(|uia| {
            Arc::new(WindowsTreeProvider { uia: uia.clone() }) as Arc<dyn bit_sr_core::tree::TreeProvider>
        })
    }

    /// Cleanly terminates all hooks and event listeners.
    pub fn stop(&mut self) {
        log::info!("Stopping Windows accessibility platform...");
        if let Some(hook) = self.keyboard_hook.take() {
            hook.stop();
        }
        if let Some(hook) = self.msaa_hook.take() {
            hook.stop();
        }
        if let Some(uia) = self.uia_client.take() {
            uia.remove_all_event_handlers();
        }
        let _ = self.com_guard.take();
    }
}

/// Windows UIA implementation of TreeProvider for harvesting accessible subtrees in web documents.
pub struct WindowsTreeProvider {
    uia: Arc<UiaClient>,
}

impl bit_sr_core::tree::TreeProvider for WindowsTreeProvider {
    fn harvest_tree(&self, max_depth: usize, max_nodes: usize) -> Option<bit_sr_core::tree::AccessibilityTree> {
        let nav = self.uia.control_view_navigator().ok()?;
        let focused = self.uia.get_focused_element().ok()?;
        let doc_root = nav.find_enclosing_document(focused.raw());
        Some(nav.harvest_subtree(&doc_root, max_depth, max_nodes))
    }
}

impl Drop for WindowsPlatform {
    fn drop(&mut self) {
        self.stop();
    }
}
