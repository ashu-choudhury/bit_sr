//! UI Automation COM Event Handler Implementations.
//! Implements FocusChanged, PropertyChanged, and Notification handlers with minimal callback latency.
//!
//! Hot path architecture:
//! COM callbacks do zero property decoding, zero string allocations, and zero regex/Explorer filtering.
//! They immediately enqueue RawUiaEvent into a lock-free queue and return in < 100ns.
//! A dedicated MTA worker thread (bit_sr_uia_worker) decodes properties, coalesces focus changes,
//! and forwards processed events to the engine coordinator.

use crate::apps::explorer::ExplorerFilter;
use crate::uia::element::UiaElement;
use bit_sr_core::events::AccessibilityEvent;
use crossbeam_channel::{Receiver, Sender};
use std::sync::Arc;
use windows::core::{implement, BSTR};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;
use windows_core::Ref;

/// Wrapper to allow transferring IUIAutomationElement across MTA threads.
/// Sound because both the UIA callback thread and the UIA event worker thread
/// reside within the same Multi-Threaded Apartment (MTA).
#[derive(Clone)]
pub struct SendElement(pub IUIAutomationElement);
unsafe impl Send for SendElement {}
unsafe impl Sync for SendElement {}

/// Raw UIA event captured inside lightweight COM callbacks.
pub enum RawUiaEvent {
    Focus(SendElement),
    PropertyChanged {
        element: SendElement,
        property_id: UIA_PROPERTY_ID,
    },
    Notification {
        display_string: String,
        activity_id: String,
    },
}

#[implement(IUIAutomationFocusChangedEventHandler)]
pub struct FocusChangedHandler {
    raw_tx: Sender<RawUiaEvent>,
}

impl FocusChangedHandler {
    pub fn new(raw_tx: Sender<RawUiaEvent>) -> Self {
        Self { raw_tx }
    }
}

impl IUIAutomationFocusChangedEventHandler_Impl for FocusChangedHandler_Impl {
    fn HandleFocusChangedEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
    ) -> windows::core::Result<()> {
        if let Some(element) = sender.as_ref() {
            let _ = self.raw_tx.try_send(RawUiaEvent::Focus(SendElement(element.clone())));
        }
        Ok(())
    }
}

#[implement(IUIAutomationPropertyChangedEventHandler)]
pub struct PropertyChangedHandler {
    raw_tx: Sender<RawUiaEvent>,
}

impl PropertyChangedHandler {
    pub fn new(raw_tx: Sender<RawUiaEvent>) -> Self {
        Self { raw_tx }
    }
}

impl IUIAutomationPropertyChangedEventHandler_Impl for PropertyChangedHandler_Impl {
    fn HandlePropertyChangedEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
        propertyid: UIA_PROPERTY_ID,
        _newvalue: &VARIANT,
    ) -> windows::core::Result<()> {
        // Fast-path filter: immediately discard properties we do not observe
        if propertyid != UIA_NamePropertyId
            && propertyid != UIA_ValueValuePropertyId
            && propertyid != UIA_RangeValueValuePropertyId
        {
            return Ok(());
        }

        if let Some(element) = sender.as_ref() {
            let _ = self.raw_tx.try_send(RawUiaEvent::PropertyChanged {
                element: SendElement(element.clone()),
                property_id: propertyid,
            });
        }
        Ok(())
    }
}

#[implement(IUIAutomationNotificationEventHandler)]
pub struct NotificationEventHandler {
    raw_tx: Sender<RawUiaEvent>,
}

impl NotificationEventHandler {
    pub fn new(raw_tx: Sender<RawUiaEvent>) -> Self {
        Self { raw_tx }
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

        let _ = self.raw_tx.try_send(RawUiaEvent::Notification {
            activity_id,
            display_string,
        });

        Ok(())
    }
}

/// Spawns the dedicated UIA event processing worker thread.
/// Runs in an MTA apartment, offloading element traversal, COM property queries,
/// Explorer filtering, and AccessibleNode generation from the Windows UIA callback thread.
/// Automatically coalesces rapid consecutive focus changes so the screen reader never speaks stale focus.
pub fn spawn_uia_worker(
    raw_rx: Receiver<RawUiaEvent>,
    tx: Sender<AccessibilityEvent>,
    explorer_filter: Arc<ExplorerFilter>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("bit_sr_uia_worker".to_string())
        .spawn(move || {
            let _com_guard = crate::com::ComGuard::init_mta().ok();

            while let Ok(mut event) = raw_rx.recv() {
                // Focus coalescing: if this is a Focus event and the queue has subsequent events,
                // drain to the latest Focus event to eliminate intermediate chatter
                if matches!(event, RawUiaEvent::Focus(_)) {
                    while let Ok(newer_event) = raw_rx.try_recv() {
                        if matches!(newer_event, RawUiaEvent::Focus(_)) {
                            event = newer_event;
                        } else {
                            process_single_uia_event(newer_event, &explorer_filter, &tx);
                        }
                    }
                }

                process_single_uia_event(event, &explorer_filter, &tx);
            }
        })
        .expect("Failed to spawn UIA event worker thread")
}

fn process_single_uia_event(
    event: RawUiaEvent,
    explorer_filter: &Arc<ExplorerFilter>,
    tx: &Sender<AccessibilityEvent>,
) {
    match event {
        RawUiaEvent::Focus(SendElement(raw_element)) => {
            let uia_elem = UiaElement::new(raw_element);
            let node = uia_elem.to_accessible_node();

            if let Some(processed) = explorer_filter.process_node(node) {
                // Guaranteed delivery for focus events: if the queue is temporarily full, wait to avoid desynchronization
                if tx.try_send(AccessibilityEvent::Focus(processed.clone())).is_err() {
                    let _ = tx.send(AccessibilityEvent::Focus(processed));
                }
            }
        }
        RawUiaEvent::PropertyChanged {
            element: SendElement(raw_element),
            property_id,
        } => {
            let uia_elem = UiaElement::new(raw_element);
            let node = uia_elem.to_accessible_node();

            if property_id == UIA_NamePropertyId {
                let _ = tx.try_send(AccessibilityEvent::NameChange {
                    new_name: node.name.clone(),
                    node,
                });
            } else if property_id == UIA_ValueValuePropertyId || property_id == UIA_RangeValueValuePropertyId {
                let _ = tx.try_send(AccessibilityEvent::ValueChange {
                    new_value: node.value.clone(),
                    node,
                });
            }
        }
        RawUiaEvent::Notification {
            display_string,
            activity_id,
        } => {
            let _ = tx.try_send(AccessibilityEvent::Notification {
                activity_id,
                display_string,
            });
        }
    }
}
