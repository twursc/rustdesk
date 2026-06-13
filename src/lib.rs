// OHOS v0.1: 把 ohos 与 android/ios 一并排除桌面专属模块，src/platform/ohos.rs
// 提供 stub 平台层，确保 lib 可编但不引入桌面 X11/D-Bus/wayland 依赖。
#[cfg(target_env = "ohos")]
#[path = "keyboard_ohos.rs"]
mod keyboard;
#[cfg(not(target_env = "ohos"))]
mod keyboard;
/// cbindgen:ignore
pub mod platform;
#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
pub use platform::{
    clip_cursor, get_cursor, get_cursor_data, get_cursor_pos, get_focused_display,
    set_cursor_pos, start_os_service,
};
// OHOS v0.1: server 子系统全部排除（被控端 v0.2 再做）
#[cfg(not(any(target_os = "ios", target_env = "ohos")))]
/// cbindgen:ignore
mod server;
#[cfg(not(any(target_os = "ios", target_env = "ohos")))]
pub use self::server::*;
mod client;
// OHOS 远端音频播放：rust → libohaudio.so OH_AudioRenderer thin FFI 桥。
#[cfg(target_env = "ohos")]
mod audio_ohos;
mod lan;
// OHOS v0.1: 不接入 rendezvous（peer discovery），直接连远端地址
#[cfg(not(any(target_os = "ios", target_env = "ohos")))]
mod rendezvous_mediator;
#[cfg(not(any(target_os = "ios", target_env = "ohos")))]
pub use self::rendezvous_mediator::*;
/// cbindgen:ignore
pub mod common;
// OHOS v0.1: 单进程，不需要桌面 IPC
#[cfg(not(any(target_os = "ios", target_env = "ohos")))]
pub mod ipc;
#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_env = "ohos",
    feature = "cli",
    feature = "flutter"
)))]
pub mod ui;
mod version;
pub use version::*;
#[cfg(any(target_os = "android", target_os = "ios", feature = "flutter"))]
mod bridge_generated;
#[cfg(any(target_os = "android", target_os = "ios", feature = "flutter"))]
pub mod flutter;
#[cfg(any(target_os = "android", target_os = "ios", feature = "flutter"))]
pub mod flutter_ffi;
use common::*;
mod auth_2fa;
#[cfg(feature = "cli")]
pub mod cli;
#[cfg(not(any(target_os = "ios", target_env = "ohos")))]
mod clipboard;
#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos", feature = "cli")))]
pub mod core_main;
mod custom_server;
mod lang;
#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
mod port_forward;

#[cfg(all(feature = "flutter", feature = "plugin_framework"))]
#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
pub mod plugin;

#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
mod tray;

#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
mod whiteboard;

#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
mod updater;

// ui_cm_interface = Connection Manager UI (被控端 CM 弹窗)，OHOS v0.1 用 stub
#[cfg(target_env = "ohos")]
#[path = "ui_cm_interface_ohos.rs"]
mod ui_cm_interface;
#[cfg(not(target_env = "ohos"))]
mod ui_cm_interface;
mod ui_interface;
mod ui_session_interface;

mod hbbs_http;

#[cfg(all(any(target_os = "windows", target_os = "linux", target_os = "macos"), not(target_env = "ohos")))]
pub mod clipboard_file;

pub mod privacy_mode;

#[cfg(windows)]
pub mod virtual_display_manager;

#[cfg(target_env = "ohos")]
#[path = "kcp_stream_ohos.rs"]
mod kcp_stream;
#[cfg(not(target_env = "ohos"))]
mod kcp_stream;
