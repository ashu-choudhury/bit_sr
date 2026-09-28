//! Microsoft UI Automation Client Subsystem.
//! Implements Sections 3.1 - 3.5 of WINDOWS.md.

pub mod cache;
pub mod element;
pub mod events;
pub mod patterns;
pub mod tree;

pub use cache::create_base_cache_request;
pub use element::{uia_control_type_to_role, UiaElement};
pub use events::{FocusChangedHandler, NotificationEventHandler, PropertyChangedHandler};
pub use patterns::Patterns;
pub use tree::TreeNavigator;

use crate::apps::explorer::ExplorerFilter;
use crate::error::Result;
use bit_sr_core::events::AccessibilityEvent;
use crossbeam_channel::Sender;
use std::sync::Arc;
use windows::core::Interface;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Accessibility::*;

pub struct UiaClient {
    client: IUIAutomation,
    cache_request: IUIAutomationCacheRequest,
    explorer_filter: Arc<ExplorerFilter>,
}

impl UiaClient {
    /// Initializes UI Automation, cleans up ProxyFactoryMapping, and registers event handlers.
    pub fn new(
        tx: Sender<AccessibilityEvent>,
        explorer_filter: Arc<ExplorerFilter>,
    ) -> Result<Self> {
        unsafe {
            // Instantiate CUIAutomation8 (Windows 8.1+ through Windows 11)
            let client: IUIAutomation = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;

            // Section 3.2: Clear WinEvents from ProxyFactoryMapping (Bugfix #7345)
            if let Ok(pfm) = client.ProxyFactoryMapping() {
                if let Ok(count) = pfm.Count() {
                    for i in 0..count {
                        if let Ok(entry) = pfm.GetEntry(i) {
                            let _ = entry.SetWinEventsForAutomationEvent(
                                UIA_AutomationPropertyChangedEventId,
                                UIA_NamePropertyId,
                                std::ptr::null(),
                            );
                        }
                    }
                }
            }

            // Section 3.3: Enable modern performance features if IUIAutomation6 is supported
            if let Ok(client6) = client.cast::<IUIAutomation6>() {
                let _ = client6.SetCoalesceEvents(CoalesceEventsOptions_Enabled);
                let _ = client6.SetConnectionRecoveryBehavior(ConnectionRecoveryBehaviorOptions_Enabled);
                log::info!("UI Automation 6 features enabled (Event Coalescing & Recovery)");
            }

            // Section 3.4: Build 18-property base CacheRequest
            let cache_request = create_base_cache_request(&client)?;

            // Register FocusChangedEventHandler
            let focus_handler: IUIAutomationFocusChangedEventHandler =
                FocusChangedHandler::new(tx.clone(), explorer_filter.clone()).into();
            client.AddFocusChangedEventHandler(&cache_request, &focus_handler)?;

            // Register PropertyChangedEventHandler for Subtree using native array
            let root = client.GetRootElementBuildCache(&cache_request)?;
            let prop_handler: IUIAutomationPropertyChangedEventHandler =
                PropertyChangedHandler::new(tx.clone()).into();
            let properties = [UIA_NamePropertyId, UIA_ValueValuePropertyId, UIA_RangeValueValuePropertyId];
            if let Err(e) = client.AddPropertyChangedEventHandlerNativeArray(
                &root,
                TreeScope_Subtree,
                &cache_request,
                &prop_handler,
                &properties,
            ) {
                log::debug!("Subtree PropertyChangedEventHandler not supported on desktop root ({:?})", e);
            }

            // Register NotificationEventHandler if supported (Windows 10 1709+)
            if let Ok(client5) = client.cast::<IUIAutomation5>() {
                let notification_handler: IUIAutomationNotificationEventHandler =
                    NotificationEventHandler::new(tx).into();
                let _ = client5.AddNotificationEventHandler(
                    &root,
                    TreeScope_Subtree,
                    &cache_request,
                    &notification_handler,
                );
            }

            Ok(Self {
                client,
                cache_request,
                explorer_filter,
            })
        }
    }

    /// Returns the active ExplorerFilter instance.
    pub fn explorer_filter(&self) -> &Arc<ExplorerFilter> {
        &self.explorer_filter
    }

    /// Queries whether a given HWND natively implements UI Automation without OS proxying.
    pub fn has_server_side_provider(&self, hwnd: HWND) -> bool {
        unsafe { UiaHasServerSideProvider(hwnd).as_bool() }
    }

    /// Obtains a ControlView tree walker for structural tree navigation.
    pub fn control_view_navigator(&self) -> Result<TreeNavigator> {
        unsafe {
            let walker = self.client.ControlViewWalker()?;
            Ok(TreeNavigator::new(walker, self.cache_request.clone()))
        }
    }

    /// Obtains a RawView tree walker for comprehensive element navigation.
    pub fn raw_view_navigator(&self) -> Result<TreeNavigator> {
        unsafe {
            let walker = self.client.RawViewWalker()?;
            Ok(TreeNavigator::new(walker, self.cache_request.clone()))
        }
    }

    /// Retrieves the current focused element with pre-cached properties.
    pub fn get_focused_element(&self) -> Result<UiaElement> {
        unsafe {
            let elem = self.client.GetFocusedElementBuildCache(&self.cache_request)?;
            Ok(UiaElement::new(elem))
        }
    }

    /// Unregisters all event handlers cleanly.
    pub fn remove_all_event_handlers(&self) {
        unsafe {
            let _ = self.client.RemoveAllEventHandlers();
        }
    }
}

impl Drop for UiaClient {
    fn drop(&mut self) {
        self.remove_all_event_handlers();
    }
}
