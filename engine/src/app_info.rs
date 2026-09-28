use std::sync::OnceLock;

/// Build-time identity of the binary hosting the engine, supplied by the launcher at startup.
#[derive(Clone, Debug)]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub authors: &'static str,
}

static APP_INFO: OnceLock<AppInfo> = OnceLock::new();

/// Register the host application's identity before any config, high score or menu code runs;
/// later calls are ignored.
pub fn init(info: AppInfo) {
    let _ = APP_INFO.set(info);
}

pub fn get() -> &'static AppInfo {
    APP_INFO
        .get()
        .expect("engine::app_info::init must be called before using the engine")
}
