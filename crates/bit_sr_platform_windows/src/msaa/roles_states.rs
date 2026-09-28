//! MSAA Role and State Conversions to Unified Types.
//! Aligned with WINDOWS.md Section 4.4 and 4.5.

use bit_sr_core::roles::Role;
use bit_sr_core::states::State;

// Standard MSAA Role constants from oleacc.h
pub const ROLE_SYSTEM_TITLEBAR: u32 = 1;
pub const ROLE_SYSTEM_MENUBAR: u32 = 2;
pub const ROLE_SYSTEM_SCROLLBAR: u32 = 3;
pub const ROLE_SYSTEM_ALERT: u32 = 8;
pub const ROLE_SYSTEM_WINDOW: u32 = 9;
pub const ROLE_SYSTEM_CLIENT: u32 = 10;
pub const ROLE_SYSTEM_MENUPOPUP: u32 = 11;
pub const ROLE_SYSTEM_MENUITEM: u32 = 12;
pub const ROLE_SYSTEM_TOOLTIP: u32 = 13;
pub const ROLE_SYSTEM_APPLICATION: u32 = 14;
pub const ROLE_SYSTEM_DOCUMENT: u32 = 15;
pub const ROLE_SYSTEM_PANE: u32 = 16;
pub const ROLE_SYSTEM_DIALOG: u32 = 18;
pub const ROLE_SYSTEM_GROUPING: u32 = 20;
pub const ROLE_SYSTEM_SEPARATOR: u32 = 21;
pub const ROLE_SYSTEM_TOOLBAR: u32 = 22;
pub const ROLE_SYSTEM_STATUSBAR: u32 = 23;
pub const ROLE_SYSTEM_TABLE: u32 = 24;
pub const ROLE_SYSTEM_COLUMNHEADER: u32 = 25;
pub const ROLE_SYSTEM_ROWHEADER: u32 = 26;
pub const ROLE_SYSTEM_CELL: u32 = 29;
pub const ROLE_SYSTEM_LINK: u32 = 30;
pub const ROLE_SYSTEM_LIST: u32 = 33;
pub const ROLE_SYSTEM_LISTITEM: u32 = 34;
pub const ROLE_SYSTEM_OUTLINE: u32 = 35;
pub const ROLE_SYSTEM_OUTLINEITEM: u32 = 36;
pub const ROLE_SYSTEM_PAGETAB: u32 = 37;
pub const ROLE_SYSTEM_PROPERTYPAGE: u32 = 38;
pub const ROLE_SYSTEM_GRAPHIC: u32 = 40;
pub const ROLE_SYSTEM_STATICTEXT: u32 = 41;
pub const ROLE_SYSTEM_TEXT: u32 = 42;
pub const ROLE_SYSTEM_PUSHBUTTON: u32 = 43;
pub const ROLE_SYSTEM_CHECKBUTTON: u32 = 44;
pub const ROLE_SYSTEM_RADIOBUTTON: u32 = 45;
pub const ROLE_SYSTEM_COMBOBOX: u32 = 46;
pub const ROLE_SYSTEM_PROGRESSBAR: u32 = 48;
pub const ROLE_SYSTEM_SLIDER: u32 = 49;
pub const ROLE_SYSTEM_SPINBUTTON: u32 = 50;
pub const ROLE_SYSTEM_PAGETABLIST: u32 = 60;
pub const ROLE_SYSTEM_SPLITBUTTON: u32 = 62;

pub fn msaa_role_to_role(msaa_role: u32) -> Role {
    match msaa_role {
        ROLE_SYSTEM_TITLEBAR => Role::TitleBar,
        ROLE_SYSTEM_MENUBAR => Role::MenuBar,
        ROLE_SYSTEM_SCROLLBAR => Role::ScrollBar,
        ROLE_SYSTEM_ALERT => Role::Alert,
        ROLE_SYSTEM_WINDOW => Role::Window,
        ROLE_SYSTEM_CLIENT | ROLE_SYSTEM_PANE => Role::Pane,
        ROLE_SYSTEM_MENUPOPUP => Role::PopupMenu,
        ROLE_SYSTEM_MENUITEM => Role::MenuItem,
        ROLE_SYSTEM_TOOLTIP => Role::ToolTip,
        ROLE_SYSTEM_APPLICATION => Role::Application,
        ROLE_SYSTEM_DOCUMENT => Role::Document,
        ROLE_SYSTEM_DIALOG => Role::Dialog,
        ROLE_SYSTEM_GROUPING => Role::Grouping,
        ROLE_SYSTEM_SEPARATOR => Role::Separator,
        ROLE_SYSTEM_TOOLBAR => Role::ToolBar,
        ROLE_SYSTEM_STATUSBAR => Role::StatusBar,
        ROLE_SYSTEM_TABLE => Role::Table,
        ROLE_SYSTEM_COLUMNHEADER => Role::TableColumnHeader,
        ROLE_SYSTEM_ROWHEADER => Role::TableRowHeader,
        ROLE_SYSTEM_CELL => Role::TableCell,
        ROLE_SYSTEM_LINK => Role::Link,
        ROLE_SYSTEM_LIST => Role::List,
        ROLE_SYSTEM_LISTITEM => Role::ListItem,
        ROLE_SYSTEM_OUTLINE => Role::TreeView,
        ROLE_SYSTEM_OUTLINEITEM => Role::TreeViewItem,
        ROLE_SYSTEM_PAGETAB => Role::Tab,
        ROLE_SYSTEM_PROPERTYPAGE => Role::PropertyPage,
        ROLE_SYSTEM_GRAPHIC => Role::Graphic,
        ROLE_SYSTEM_STATICTEXT => Role::StaticText,
        ROLE_SYSTEM_TEXT => Role::EditableText,
        ROLE_SYSTEM_PUSHBUTTON => Role::Button,
        ROLE_SYSTEM_CHECKBUTTON => Role::CheckBox,
        ROLE_SYSTEM_RADIOBUTTON => Role::RadioButton,
        ROLE_SYSTEM_COMBOBOX => Role::ComboBox,
        ROLE_SYSTEM_PROGRESSBAR => Role::ProgressBar,
        ROLE_SYSTEM_SLIDER => Role::Slider,
        ROLE_SYSTEM_SPINBUTTON => Role::SpinButton,
        ROLE_SYSTEM_PAGETABLIST => Role::TabControl,
        ROLE_SYSTEM_SPLITBUTTON => Role::SplitButton,
        _ => Role::Unknown,
    }
}

// Standard MSAA State bitflags from oleacc.h
pub const STATE_SYSTEM_UNAVAILABLE: u32 = 0x00000001;
pub const STATE_SYSTEM_SELECTED: u32 = 0x00000002;
pub const STATE_SYSTEM_FOCUSED: u32 = 0x00000004;
pub const STATE_SYSTEM_PRESSED: u32 = 0x00000008;
pub const STATE_SYSTEM_CHECKED: u32 = 0x00000010;
pub const STATE_SYSTEM_MIXED: u32 = 0x00000020;
pub const STATE_SYSTEM_READONLY: u32 = 0x00000040;
pub const STATE_SYSTEM_HOTTRACKED: u32 = 0x00000080;
pub const STATE_SYSTEM_EXPANDED: u32 = 0x00000200;
pub const STATE_SYSTEM_COLLAPSED: u32 = 0x00000400;
pub const STATE_SYSTEM_BUSY: u32 = 0x00000800;
pub const STATE_SYSTEM_INVISIBLE: u32 = 0x00004000;
pub const STATE_SYSTEM_OFFSCREEN: u32 = 0x00008000;
pub const STATE_SYSTEM_FOCUSABLE: u32 = 0x00040000;
pub const STATE_SYSTEM_SELECTABLE: u32 = 0x00080000;
pub const STATE_SYSTEM_LINKED: u32 = 0x00100000;
pub const STATE_SYSTEM_TRAVERSED: u32 = 0x00200000;
pub const STATE_SYSTEM_MULTISELECTABLE: u32 = 0x00400000;
pub const STATE_SYSTEM_PROTECTED: u32 = 0x02000000;
pub const STATE_SYSTEM_HASPOPUP: u32 = 0x40000000;

pub fn msaa_state_to_state(msaa_state: u32) -> State {
    let mut state = State::empty();
    if (msaa_state & STATE_SYSTEM_UNAVAILABLE) != 0 {
        state |= State::UNAVAILABLE;
    }
    if (msaa_state & STATE_SYSTEM_SELECTED) != 0 {
        state |= State::SELECTED;
    }
    if (msaa_state & STATE_SYSTEM_FOCUSED) != 0 {
        state |= State::FOCUSED;
    }
    if (msaa_state & STATE_SYSTEM_PRESSED) != 0 {
        state |= State::PRESSED;
    }
    if (msaa_state & STATE_SYSTEM_CHECKED) != 0 {
        state |= State::CHECKED;
    }
    if (msaa_state & STATE_SYSTEM_MIXED) != 0 {
        state |= State::HALFCHECKED;
    }
    if (msaa_state & STATE_SYSTEM_READONLY) != 0 {
        state |= State::READONLY;
    }
    if (msaa_state & STATE_SYSTEM_HOTTRACKED) != 0 {
        state |= State::HOTTRACKED;
    }
    if (msaa_state & STATE_SYSTEM_EXPANDED) != 0 {
        state |= State::EXPANDED;
    }
    if (msaa_state & STATE_SYSTEM_COLLAPSED) != 0 {
        state |= State::COLLAPSED;
    }
    if (msaa_state & STATE_SYSTEM_BUSY) != 0 {
        state |= State::BUSY;
    }
    if (msaa_state & STATE_SYSTEM_INVISIBLE) != 0 {
        state |= State::INVISIBLE;
    }
    if (msaa_state & STATE_SYSTEM_OFFSCREEN) != 0 {
        state |= State::OFFSCREEN;
    }
    if (msaa_state & STATE_SYSTEM_FOCUSABLE) != 0 {
        state |= State::FOCUSABLE;
    }
    if (msaa_state & STATE_SYSTEM_SELECTABLE) != 0 {
        state |= State::SELECTABLE;
    }
    if (msaa_state & STATE_SYSTEM_LINKED) != 0 {
        state |= State::LINKED;
    }
    if (msaa_state & STATE_SYSTEM_TRAVERSED) != 0 {
        state |= State::VISITED;
    }
    if (msaa_state & STATE_SYSTEM_MULTISELECTABLE) != 0 {
        state |= State::MULTISELECTABLE;
    }
    if (msaa_state & STATE_SYSTEM_PROTECTED) != 0 {
        state |= State::PROTECTED;
    }
    if (msaa_state & STATE_SYSTEM_HASPOPUP) != 0 {
        state |= State::HASPOPUP;
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msaa_role_conversion() {
        assert_eq!(msaa_role_to_role(ROLE_SYSTEM_PUSHBUTTON), Role::Button);
        assert_eq!(msaa_role_to_role(ROLE_SYSTEM_CHECKBUTTON), Role::CheckBox);
        assert_eq!(msaa_role_to_role(ROLE_SYSTEM_LISTITEM), Role::ListItem);
        assert_eq!(msaa_role_to_role(ROLE_SYSTEM_OUTLINEITEM), Role::TreeViewItem);
        assert_eq!(msaa_role_to_role(9999), Role::Unknown);
    }

    #[test]
    fn test_msaa_state_conversion() {
        let raw = STATE_SYSTEM_FOCUSED | STATE_SYSTEM_SELECTED | STATE_SYSTEM_CHECKED;
        let converted = msaa_state_to_state(raw);
        assert!(converted.contains(State::FOCUSED));
        assert!(converted.contains(State::SELECTED));
        assert!(converted.contains(State::CHECKED));
        assert!(!converted.contains(State::EXPANDED));
    }
}
