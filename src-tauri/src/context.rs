#[cfg(not(target_os = "macos"))]
use crate::modes::CapturedContext;
use crate::modes::ResolvedContext;
use std::sync::Mutex;
use std::thread::JoinHandle;
use tauri::{AppHandle, Manager};

#[cfg(target_os = "macos")]
#[path = "context_macos.rs"]
mod platform;

#[cfg(target_os = "macos")]
pub(crate) fn focused_control_is_nontext() -> bool {
    platform::focused_control_is_nontext()
}

#[derive(Default)]
pub struct SessionContext(Mutex<Option<JoinHandle<ResolvedContext>>>);

pub fn begin(app: &AppHandle) {
    let modes = crate::settings::get_settings(app).modes;
    #[cfg(target_os = "macos")]
    let (identity, pid) = platform::frontmost();
    #[cfg(not(target_os = "macos"))]
    let identity = CapturedContext::default();
    let app_clone = app.clone();
    let worker = std::thread::spawn(move || {
        #[cfg(target_os = "macos")]
        {
            {
                use tauri_plugin_clipboard_manager::ClipboardExt;
                platform::capture(identity, pid, &modes, || {
                    app_clone.clipboard().read_text().ok()
                })
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = app_clone;
            ResolvedContext {
                mode: crate::modes::match_mode(&modes, &identity),
                context: identity,
            }
        }
    });
    *app.state::<SessionContext>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(worker);
}

pub fn take(app: &AppHandle) -> Option<JoinHandle<ResolvedContext>> {
    app.state::<SessionContext>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}

pub async fn resolve(worker: Option<JoinHandle<ResolvedContext>>) -> ResolvedContext {
    match worker {
        Some(worker) => {
            tauri::async_runtime::spawn_blocking(move || worker.join().unwrap_or_default())
                .await
                .unwrap_or_default()
        }
        None => ResolvedContext::default(),
    }
}
