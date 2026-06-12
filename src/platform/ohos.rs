// OHOS 平台层 stub。v0.1 主控端不调用大多数 platform API；这里只提供
// 编译时通过的最小符号集（多数实现是空/默认）。后续若需被控端补完。
//
// 注意：能不导出的就不导出；其它模块通过 cfg 排除路径访问。

use hbb_common::ResultType;

pub const PA_SAMPLE_RATE: u32 = 48000;

pub fn is_root() -> bool {
    false
}

pub fn is_installed() -> bool {
    false
}

pub fn is_prelogin() -> bool {
    false
}

pub fn is_x11() -> bool {
    false
}

pub fn get_active_username() -> String {
    "ohos".into()
}

pub fn lock_screen() {}

pub fn send_sas() {}

pub fn try_kill_broker() {}

pub fn try_kill_rustdesk_main_window_process() -> ResultType<()> {
    Ok(())
}

pub fn check_super_user_permission() -> ResultType<bool> {
    Ok(false)
}

#[derive(Default)]
pub struct WakeLock;

impl WakeLock {
    pub fn new(_tag: &str) -> Self {
        Self
    }
}
