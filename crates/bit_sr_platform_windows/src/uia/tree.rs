//! UI Automation Tree Walker Navigation.
//! Aligned with WINDOWS.md Section 3.4.

use windows::Win32::UI::Accessibility::{IUIAutomationCacheRequest, IUIAutomationElement, IUIAutomationTreeWalker};

pub struct TreeNavigator {
    walker: IUIAutomationTreeWalker,
    cache_request: IUIAutomationCacheRequest,
}

impl TreeNavigator {
    pub fn new(walker: IUIAutomationTreeWalker, cache_request: IUIAutomationCacheRequest) -> Self {
        Self { walker, cache_request }
    }

    /// Navigates up to the parent element with cached properties.
    pub fn get_parent(&self, element: &IUIAutomationElement) -> Option<IUIAutomationElement> {
        unsafe {
            self.walker
                .GetParentElementBuildCache(element, &self.cache_request)
                .ok()
        }
    }

    /// Navigates to the first child element with cached properties.
    pub fn get_first_child(&self, element: &IUIAutomationElement) -> Option<IUIAutomationElement> {
        unsafe {
            self.walker
                .GetFirstChildElementBuildCache(element, &self.cache_request)
                .ok()
        }
    }

    /// Navigates to the next sibling element with cached properties.
    pub fn get_next_sibling(&self, element: &IUIAutomationElement) -> Option<IUIAutomationElement> {
        unsafe {
            self.walker
                .GetNextSiblingElementBuildCache(element, &self.cache_request)
                .ok()
        }
    }

    /// Navigates to the previous sibling element with cached properties.
    pub fn get_previous_sibling(&self, element: &IUIAutomationElement) -> Option<IUIAutomationElement> {
        unsafe {
            self.walker
                .GetPreviousSiblingElementBuildCache(element, &self.cache_request)
                .ok()
        }
    }
}
