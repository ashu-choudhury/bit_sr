//! Low-Level Mouse Hook Subsystem (WH_MOUSE_LL).

use bit_sr_core::input::{MouseAction, MouseEvent};
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::WindowsAndMessaging::{
    MSLLHOOKSTRUCT, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

const LLMHF_INJECTED: u32 = 0x01;

/// Translates a mouse message and structure into a unified MouseEvent.
pub fn parse_mouse_event(w_param: WPARAM, msll: &MSLLHOOKSTRUCT) -> Option<MouseEvent> {
    let action = match w_param.0 as u32 {
        WM_MOUSEMOVE => MouseAction::Move,
        WM_LBUTTONDOWN => MouseAction::LeftDown,
        WM_LBUTTONUP => MouseAction::LeftUp,
        WM_RBUTTONDOWN => MouseAction::RightDown,
        WM_RBUTTONUP => MouseAction::RightUp,
        WM_MBUTTONDOWN => MouseAction::MiddleDown,
        WM_MBUTTONUP => MouseAction::MiddleUp,
        WM_MOUSEWHEEL => MouseAction::Wheel,
        _ => return None,
    };

    Some(MouseEvent {
        x: msll.pt.x,
        y: msll.pt.y,
        action,
        is_injected: (msll.flags & LLMHF_INJECTED) != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::POINT;

    #[test]
    fn test_parse_mouse_event() {
        let msll = MSLLHOOKSTRUCT {
            pt: POINT { x: 100, y: 200 },
            mouseData: 0,
            flags: 0,
            time: 0,
            dwExtraInfo: 0,
        };

        let ev = parse_mouse_event(WPARAM(WM_LBUTTONDOWN as usize), &msll).expect("event parsed");
        assert_eq!(ev.x, 100);
        assert_eq!(ev.y, 200);
        assert_eq!(ev.action, MouseAction::LeftDown);
        assert!(!ev.is_injected);

        let injected = MSLLHOOKSTRUCT {
            pt: POINT { x: 50, y: 75 },
            mouseData: 0,
            flags: LLMHF_INJECTED,
            time: 0,
            dwExtraInfo: 0,
        };

        let ev2 = parse_mouse_event(WPARAM(WM_MOUSEMOVE as usize), &injected).expect("event parsed");
        assert_eq!(ev2.action, MouseAction::Move);
        assert!(ev2.is_injected);
    }
}
