// OHOS v0.1 keyboard stub。v0.2 接 ArkTS RawKeyEvent 时再补真。
#![allow(dead_code)]
#![allow(unused_variables)]

use hbb_common::message_proto::{ControlKey, KeyEvent, KeyboardMode};

pub fn release_remote_keys(_keyboard_mode: &str) {}
pub fn set_relative_mouse_mode_state(_state: bool) {}
pub fn update_grab_get_key_name(_keyboard_mode: &str) {}

pub mod input_source {
    use hbb_common::ResultType;
    pub fn init_input_source() {}
    pub fn get_supported_input_source() -> Vec<String> { Vec::new() }
}

pub mod client {
    use super::*;
    use crate::ui_session_interface::{InvokeUiSession, Session};

    pub fn change_grab_status<T: InvokeUiSession>(_state: u32, _keyboard_mode: &str, _session_id: uuid::Uuid) {}

    pub fn event_lock_screen() -> KeyEvent {
        let mut e = KeyEvent::default();
        e.set_control_key(ControlKey::LockScreen);
        e.mode = KeyboardMode::Legacy.into();
        e
    }

    pub fn event_ctrl_alt_del() -> KeyEvent {
        let mut e = KeyEvent::default();
        e.set_control_key(ControlKey::CtrlAltDel);
        e.mode = KeyboardMode::Legacy.into();
        e
    }

    pub fn legacy_modifiers(_msg: &mut KeyEvent, _alt: bool, _ctrl: bool, _shift: bool, _command: bool) {}

    pub fn get_modifiers_state(alt: bool, ctrl: bool, shift: bool, command: bool) -> (bool, bool, bool, bool) {
        (alt, ctrl, shift, command)
    }

    pub fn process_event_with_session<T: InvokeUiSession>(
        _keyboard_mode: &str,
        _event: &(),
        _lock_modes: Option<i32>,
        _session: &Session<T>,
    ) {}
}
