//! UI Automation COM Event Handler Implementations.
//! Implements FocusChanged, PropertyChanged, and Notification handlers.

use crate::apps::explorer::ExplorerFilter;
use crate::uia::element::UiaElement;
use bit_sr_core::events::AccessibilityEvent;
use crossbeam_channel::Sender;
use std::sync::Arc;
use windows::core::{implement, BSTR};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;
use windows_core::Ref;

#[implement(IUIAutomationFocusChangedEventHandler)]
pub struct FocusChangedHandler {
    tx: Sender<AccessibilityEvent>,
    explorer_filter: Arc<ExplorerFilter>,
}

impl FocusChangedHandler {
    pub fn new(tx: Sender<AccessibilityEvent>, explorer_filter: Arc<ExplorerFilter>) -> Self {
        Self { tx, explorer_filter }
    }
}

impl IUIAutomationFocusChangedEventHandler_Impl for FocusChangedHandler_Impl {
    fn HandleFocusChangedEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
    ) -> windows::core::Result<()> {
        if let Some(element) = sender.as_ref() {
            let uia_elem = UiaElement::new(element.clone());
            let node = uia_elem.to_accessible_node();

            // Apply Explorer filtering & sanitization if in explorer.exe
            if let Some(processed) = self.explorer_filter.process_node(node) {
                let _ = self.tx.try_send(AccessibilityEvent::Focus(processed));
            }
        }
        Ok(())
    }
}

#[implement(IUIAutomationPropertyChangedEventHandler)]
pub struct PropertyChangedHandler {
    tx: Sender<AccessibilityEvent>,
}

impl PropertyChangedHandler {
    pub fn new(tx: Sender<AccessibilityEvent>) -> Self {
        Self { tx }
    }
}

impl IUIAutomationPropertyChangedEventHandler_Impl for PropertyChangedHandler_Impl {
    fn HandlePropertyChangedEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
        propertyid: UIA_PROPERTY_ID,
        _newvalue: &VARIANT,
    ) -> windows::core::Result<()> {
        if let Some(element) = sender.as_ref() {
            let uia_elem = UiaElement::new(element.clone());
            let node = uia_elem.to_accessible_node();

            if propertyid == UIA_NamePropertyId {
                let _ = self.tx.try_send(AccessibilityEvent::NameChange {
                    new_name: node.name.clone(),
                    node,
                });
            } else if propertyid == UIA_ValueValuePropertyId || propertyid == UIA_RangeValueValuePropertyId {
                let _ = self.tx.try_send(AccessibilityEvent::ValueChange {
                    new_value: node.value.clone(),
                    node,
                });
            }
        }
        Ok(())
    }
}

#[implement(IUIAutomationNotificationEventHandler)]
pub struct NotificationEventHandler {
    tx: Sender<AccessibilityEvent>,
}

impl NotificationEventHandler {
    pub fn new(tx: Sender<AccessibilityEvent>) -> Self {
        Self { tx }
    }
}

impl IUIAutomationNotificationEventHandler_Impl for NotificationEventHandler_Impl {
    fn HandleNotificationEvent(
        &self,
        _sender: Ref<'_, IUIAutomationElement>,
        _notificationkind: NotificationKind,
        _notificationprocessing: NotificationProcessing,
        displaystring: &BSTR,
        activityid: &BSTR,
    ) -> windows::core::Result<()> {
        let display_string = displaystring.to_string();
        let activity_id = activityid.to_string();

        let _ = self.tx.try_send(AccessibilityEvent::Notification {
            activity_id,
            display_string,
        });

        Ok(())
    }
}
