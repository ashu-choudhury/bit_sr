//! MSAA and WinEvents Subsystem (oleacc.dll).
//! Aligned with WINDOWS.md Section 2.3 and Section 4.

pub mod accessible;
pub mod roles_states;

pub use accessible::MsaaElement;

use bit_sr_core::events::AccessibilityEvent;
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    AccessibleObjectFromEvent, SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK, IAccessible,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, PostThreadMessageW, MSG, WINEVENT_OUTOFCONTEXT, WM_QUIT,
};

static MSAA_CHANNEL: Mutex<Option<Sender<AccessibilityEvent>>> = Mutex::new(None);
static IS_RUNNING: AtomicBool = AtomicBool::new(false);

const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
const EVENT_OBJECT_FOCUS: u32 = 0x8005;
const EVENT_OBJECT_SELECTION: u32 = 0x8006;
const EVENT_OBJECT_STATECHANGE: u32 = 0x800A;
const EVENT_OBJECT_NAMECHANGE: u32 = 0x800C;
const EVENT_OBJECT_VALUECHANGE: u32 = 0x800E;

unsafe extern "system" fn win_event_callback(
    _h_win_event_hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if hwnd.0.is_null() {
        return;
    }

    let mut p_acc: Option<IAccessible> = None;
    let mut var_child = VARIANT::default();

    let hr = unsafe {
        AccessibleObjectFromEvent(
            hwnd,
            id_object as u32,
            id_child as u32,
            &mut p_acc as *mut _,
            &mut var_child as *mut _,
        )
    };

    if hr.is_err() {
        return;
    }

    if let Some(acc) = p_acc {
        let msaa = MsaaElement::new(acc, id_child, hwnd.0 as usize);
        let node = msaa.to_accessible_node();

        if let Ok(guard) = MSAA_CHANNEL.lock() {
            if let Some(ref tx) = *guard {
                match event {
                    EVENT_OBJECT_FOCUS | EVENT_SYSTEM_FOREGROUND => {
                        let _ = tx.try_send(AccessibilityEvent::Focus(node));
                    }
                    EVENT_OBJECT_SELECTION => {
                        let _ = tx.try_send(AccessibilityEvent::Selection(node));
                    }
                    EVENT_OBJECT_NAMECHANGE => {
                        let _ = tx.try_send(AccessibilityEvent::NameChange {
                            new_name: node.name.clone(),
                            node,
                        });
                    }
                    EVENT_OBJECT_STATECHANGE => {
                        let _ = tx.try_send(AccessibilityEvent::StateChange {
                            state: node.states,
                            is_set: true,
                            node,
                        });
                    }
                    EVENT_OBJECT_VALUECHANGE => {
                        let _ = tx.try_send(AccessibilityEvent::ValueChange {
                            new_value: node.value.clone(),
                            node,
                        });
                    }
                    _ => {}
                }
            }
        }
    }
}

pub struct WinEventHookHandle {
    thread_id: u32,
}

impl WinEventHookHandle {
    /// Launches the WinEvent listener on a dedicated STA thread with a message pump.
    pub fn start(tx: Sender<AccessibilityEvent>) -> Result<Self, crate::error::Error> {
        let (ready_tx, ready_rx) = crossbeam_channel::bounded(1);

        std::thread::Builder::new()
            .name("bit_sr_msaa_hook".to_string())
            .spawn(move || unsafe {
                let thread_id = windows::Win32::System::Threading::GetCurrentThreadId();
                if let Ok(mut guard) = MSAA_CHANNEL.lock() {
                    *guard = Some(tx);
                }

                let hook = SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_OBJECT_VALUECHANGE,
                    None,
                    Some(win_event_callback),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                );

                if hook.0.is_null() {
                    let _ = ready_tx.send(Err(crate::error::Error::HookInstallationFailed("SetWinEventHook failed")));
                    return;
                }

                IS_RUNNING.store(true, Ordering::SeqCst);
                let _ = ready_tx.send(Ok(thread_id));

                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    // Pump WinEvents
                }

                let _ = UnhookWinEvent(hook);
                IS_RUNNING.store(false, Ordering::SeqCst);
                if let Ok(mut guard) = MSAA_CHANNEL.lock() {
                    *guard = None;
                }
            })
            .map_err(|e| crate::error::Error::Internal(e.to_string()))?;

        let thread_id = ready_rx
            .recv()
            .map_err(|_| crate::error::Error::HookInstallationFailed("WinEvent thread failed"))??;

        Ok(Self { thread_id })
    }

    pub fn stop(self) {
        if IS_RUNNING.load(Ordering::SeqCst) {
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
    }
}
