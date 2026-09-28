//! UI Automation CacheRequest Builder.
//! Pre-populates all 18 core properties in event payloads to eliminate RPC roundtrips.
//! Aligned with WINDOWS.md Section 3.4.

use crate::error::Result;
use windows::Win32::UI::Accessibility::{
    IUIAutomation, IUIAutomationCacheRequest, UIA_AriaRolePropertyId, UIA_AutomationIdPropertyId,
    UIA_ClassNamePropertyId, UIA_ControlTypePropertyId, UIA_FrameworkIdPropertyId,
    UIA_HasKeyboardFocusPropertyId, UIA_IsContentElementPropertyId,
    UIA_IsControlElementPropertyId, UIA_IsKeyboardFocusablePropertyId,
    UIA_IsOffscreenPropertyId, UIA_IsTextPatternAvailablePropertyId, UIA_LevelPropertyId,
    UIA_LocalizedControlTypePropertyId, UIA_NamePropertyId, UIA_NativeWindowHandlePropertyId,
    UIA_PositionInSetPropertyId, UIA_ProcessIdPropertyId, UIA_SizeOfSetPropertyId,
    UIA_TextPatternId, UIA_SelectionItemPatternId, UIA_ValuePatternId,
};

pub fn create_base_cache_request(client: &IUIAutomation) -> Result<IUIAutomationCacheRequest> {
    unsafe {
        let cache = client.CreateCacheRequest()?;

        // Add 18 core properties
        cache.AddProperty(UIA_FrameworkIdPropertyId)?;
        cache.AddProperty(UIA_AutomationIdPropertyId)?;
        cache.AddProperty(UIA_ClassNamePropertyId)?;
        cache.AddProperty(UIA_ControlTypePropertyId)?;
        cache.AddProperty(UIA_NamePropertyId)?;
        cache.AddProperty(UIA_LocalizedControlTypePropertyId)?;
        cache.AddProperty(UIA_HasKeyboardFocusPropertyId)?;
        cache.AddProperty(UIA_IsKeyboardFocusablePropertyId)?;
        cache.AddProperty(UIA_IsControlElementPropertyId)?;
        cache.AddProperty(UIA_IsContentElementPropertyId)?;
        cache.AddProperty(UIA_ProcessIdPropertyId)?;
        cache.AddProperty(UIA_NativeWindowHandlePropertyId)?;
        cache.AddProperty(UIA_IsTextPatternAvailablePropertyId)?;
        cache.AddProperty(UIA_AriaRolePropertyId)?;
        cache.AddProperty(UIA_PositionInSetPropertyId)?;
        cache.AddProperty(UIA_SizeOfSetPropertyId)?;
        cache.AddProperty(UIA_LevelPropertyId)?;
        cache.AddProperty(UIA_IsOffscreenPropertyId)?;

        // Pre-cache key patterns
        let _ = cache.AddPattern(UIA_TextPatternId);
        let _ = cache.AddPattern(UIA_SelectionItemPatternId);
        let _ = cache.AddPattern(UIA_ValuePatternId);

        Ok(cache)
    }
}
