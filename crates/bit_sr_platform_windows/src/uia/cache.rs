//! UI Automation CacheRequest Builder.
//! Pre-populates all 18 core properties in event payloads to eliminate RPC roundtrips.
//! Aligned with WINDOWS.md Section 3.4.

use crate::error::Result;
use windows::Win32::UI::Accessibility::{
    IUIAutomation, IUIAutomationCacheRequest, UIA_AccessKeyPropertyId, UIA_AriaPropertiesPropertyId,
    UIA_AriaRolePropertyId, UIA_AutomationIdPropertyId, UIA_ClassNamePropertyId,
    UIA_ControlTypePropertyId, UIA_ExpandCollapseExpandCollapseStatePropertyId,
    UIA_ExpandCollapsePatternId, UIA_FrameworkIdPropertyId, UIA_FullDescriptionPropertyId,
    UIA_HasKeyboardFocusPropertyId, UIA_HelpTextPropertyId, UIA_IsContentElementPropertyId,
    UIA_IsControlElementPropertyId, UIA_IsEnabledPropertyId, UIA_IsKeyboardFocusablePropertyId,
    UIA_IsOffscreenPropertyId, UIA_IsTextPatternAvailablePropertyId, UIA_LevelPropertyId,
    UIA_LocalizedControlTypePropertyId, UIA_NamePropertyId, UIA_NativeWindowHandlePropertyId,
    UIA_PositionInSetPropertyId, UIA_ProcessIdPropertyId, UIA_RangeValueValuePropertyId,
    UIA_SelectionItemIsSelectedPropertyId, UIA_SelectionItemPatternId, UIA_SizeOfSetPropertyId,
    UIA_TextPatternId, UIA_TogglePatternId, UIA_ToggleToggleStatePropertyId,
    UIA_ValueIsReadOnlyPropertyId, UIA_ValuePatternId, UIA_ValueValuePropertyId,
};

pub fn create_base_cache_request(client: &IUIAutomation) -> Result<IUIAutomationCacheRequest> {
    unsafe {
        let cache = client.CreateCacheRequest()?;

        // Core identity & layout properties
        let _ = cache.AddProperty(UIA_FrameworkIdPropertyId);
        let _ = cache.AddProperty(UIA_AutomationIdPropertyId);
        let _ = cache.AddProperty(UIA_ClassNamePropertyId);
        let _ = cache.AddProperty(UIA_ControlTypePropertyId);
        let _ = cache.AddProperty(UIA_NamePropertyId);
        let _ = cache.AddProperty(UIA_LocalizedControlTypePropertyId);
        let _ = cache.AddProperty(UIA_HasKeyboardFocusPropertyId);
        let _ = cache.AddProperty(UIA_IsKeyboardFocusablePropertyId);
        let _ = cache.AddProperty(UIA_IsControlElementPropertyId);
        let _ = cache.AddProperty(UIA_IsContentElementPropertyId);
        let _ = cache.AddProperty(UIA_ProcessIdPropertyId);
        let _ = cache.AddProperty(UIA_NativeWindowHandlePropertyId);
        let _ = cache.AddProperty(UIA_IsTextPatternAvailablePropertyId);
        let _ = cache.AddProperty(UIA_AriaRolePropertyId);
        let _ = cache.AddProperty(UIA_AriaPropertiesPropertyId);
        let _ = cache.AddProperty(UIA_PositionInSetPropertyId);
        let _ = cache.AddProperty(UIA_SizeOfSetPropertyId);
        let _ = cache.AddProperty(UIA_LevelPropertyId);
        let _ = cache.AddProperty(UIA_IsOffscreenPropertyId);
        let _ = cache.AddProperty(UIA_IsEnabledPropertyId);
        let _ = cache.AddProperty(UIA_HelpTextPropertyId);
        let _ = cache.AddProperty(UIA_FullDescriptionPropertyId);
        let _ = cache.AddProperty(UIA_ValueValuePropertyId);
        let _ = cache.AddProperty(UIA_RangeValueValuePropertyId);
        let _ = cache.AddProperty(UIA_ValueIsReadOnlyPropertyId);
        let _ = cache.AddProperty(UIA_ToggleToggleStatePropertyId);
        let _ = cache.AddProperty(UIA_ExpandCollapseExpandCollapseStatePropertyId);
        let _ = cache.AddProperty(UIA_SelectionItemIsSelectedPropertyId);
        let _ = cache.AddProperty(UIA_AccessKeyPropertyId);

        // Pre-cache key patterns
        let _ = cache.AddPattern(UIA_TextPatternId);
        let _ = cache.AddPattern(UIA_SelectionItemPatternId);
        let _ = cache.AddPattern(UIA_ValuePatternId);
        let _ = cache.AddPattern(UIA_TogglePatternId);
        let _ = cache.AddPattern(UIA_ExpandCollapsePatternId);

        Ok(cache)
    }
}
