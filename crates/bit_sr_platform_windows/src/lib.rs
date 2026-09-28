//! Native Windows Accessibility and Input Platform Implementation.
//! 100% aligned with WINDOWS.md specification.

pub mod apps;
pub mod com;
pub mod common_controls;
pub mod desktop;
pub mod error;
pub mod input;
pub mod msaa;
pub mod uia;
pub mod watchdog;

pub use apps::ExplorerFilter;
pub use com::ComGuard;
pub use common_controls::{EditControlReader, SysListView32Reader};
pub use desktop::is_secure_desktop_active;
pub use error::{Error, Result};
pub use input::{get_current_modifiers, KeyboardHookHandle};
pub use msaa::{MsaaElement, WinEventHookHandle};
pub use uia::{create_base_cache_request, Patterns, TreeNavigator, UiaClient, UiaElement};
pub use watchdog::{is_window_hung, safe_send_message_timeout};

use bit_sr_core::events::AccessibilityEvent;
use crossbeam_channel::Sender;
use std::sync::Arc;

/// Master platform coordinator for Windows.
/// Manages the low-level keyboard hook thread, WinEvent MSAA hook thread,
/// and the dedicated UI Automation MTA worker thread.
pub struct WindowsPlatform {
    keyboard_hook: Option<KeyboardHookHandle>,
    msaa_hook: Option<WinEventHookHandle>,
    uia_client: Option<UiaClient>,
    explorer_filter: Arc<ExplorerFilter>,
}

impl WindowsPlatform {
    /// Initializes all Windows subsystems and begins streaming accessibility events.
    pub fn start(tx: Sender<AccessibilityEvent>) -> Result<Self> {
        let explorer_filter = Arc::new(ExplorerFilter::new());

        // 1. Start low-level keyboard hook on dedicated thread
        log::info!("Starting Windows low-level keyboard hook...");
        let keyboard_hook = KeyboardHookHandle::start(tx.clone())?;

        // 2. Start MSAA / WinEvents hook on dedicated thread
        log::info!("Starting Windows MSAA WinEvent hook...");
        let msaa_hook = WinEventHookHandle::start(tx.clone())?;

        // 3. Initialize MTA COM on the current/worker thread and start UIA
        log::info!("Initializing COM MTA and Microsoft UI Automation client...");
        let _ = com::ComGuard::init_mta();
        let uia_client = UiaClient::new(tx, explorer_filter.clone())?;

        log::info!("Windows accessibility platform successfully started!");

        Ok(Self {
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
    pub fn uia(&self) -> Option<&UiaClient> {
        self.uia_client.as_ref()
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
    }
}

impl Drop for WindowsPlatform {
    fn drop(&mut self) {
        self.stop();
    }
}
