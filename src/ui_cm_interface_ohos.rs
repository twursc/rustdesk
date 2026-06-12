// OHOS v0.1 主控端：Connection Manager 是被控端弹窗模块，stub 化即可。
// 仅满足 flutter_ffi 中 cm_*/handle_incoming_voice_call/get_clients_* 的符号引用。
#![allow(dead_code, unused_variables)]

pub fn has_active_clients() -> bool { false }
pub fn get_clients_state() -> String { String::new() }
pub fn get_clients_length() -> usize { 0 }
pub fn get_click_time() -> i64 { 0 }
pub fn check_click_time(_conn_id: i32) {}
pub fn authorize(_conn_id: i32) {}
pub fn close(_conn_id: i32) {}
pub fn close_voice_call(_id: i32) {}
pub fn elevate_portable(_conn_id: i32) {}
pub fn handle_incoming_voice_call(_id: i32, _accept: bool) {}
pub fn remove(_conn_id: i32) {}
pub fn send_chat(_conn_id: i32, _msg: String) {}
pub fn switch_back(_conn_id: i32) {}
pub fn switch_permission(_conn_id: i32, _name: String, _enabled: bool) {}
pub fn switch_permission_all(_name: String, _enabled: bool, _conn_type: Option<i32>) {}
pub fn can_elevate() -> bool { false }
