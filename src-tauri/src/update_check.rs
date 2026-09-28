//! Update check against the published installer.
//!
//! `https://hushtype.vercel.app/latest.json` is a static file written by
//! `scripts/package-website-release.sh` that names the version the installer
//! serves. The app reads it 30 seconds after launch and every six hours while
//! the "Check for Updates" setting is on, and on request from the tray or the
//! About settings. The request carries no identifier or query string, and the
//! response is not stored. A newer version is announced once per run by a
//! notification, a tray entry, and the About settings. Installing runs the
//! published installer in Terminal, the same path as a first install: macOS
//! ties Accessibility access to the exact build, and the installer already
//! resets that entry and reopens the app.

use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use tauri_specta::Event;

use crate::settings;
use crate::tray_i18n::{get_tray_translations, TrayStrings};

const LATEST_URL: &str = "https://hushtype.vercel.app/latest.json";
const INSTALL_COMMAND: &str = "curl -fsSL https://hushtype.vercel.app/install.sh | bash";
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// Sent to the settings window when a check finds a newer version.
#[derive(Clone, Debug, Serialize, Deserialize, Type, Event)]
pub struct UpdateAvailable {
    pub version: String,
}

#[derive(Default)]
struct Inner {
    /// Newest published version, when it is newer than this build.
    available: Option<String>,
    /// Version already announced with a notification in this run.
    announced: Option<String>,
}

/// Tauri managed state holding the last check's outcome.
#[derive(Default)]
pub struct UpdateState(Mutex<Inner>);

#[derive(Deserialize)]
struct Latest {
    version: String,
}

fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The newer published version, if the last check found one.
pub fn available_version(app: &AppHandle) -> Option<String> {
    let state = app.try_state::<UpdateState>()?;
    let inner = state.0.lock().ok()?;
    inner.available.clone()
}

/// Registers the state and starts the periodic check. The loop keeps running
/// while the setting is off, so turning it on later needs no restart.
pub fn start(app: &AppHandle) {
    app.manage(UpdateState::default());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK_DELAY).await;
        loop {
            if settings::update_checks_effectively_enabled(&settings::get_settings(&app)) {
                match check(&app).await {
                    Ok(Some(version)) => log::info!("hushtype {version} is available."),
                    Ok(None) => log::debug!("hushtype {} is current.", current_version()),
                    Err(e) => log::warn!("Update check failed: {e}"),
                }
            }
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

/// Fetches the published version and compares it with this build. A newer
/// version is kept for the tray menu, sent to the settings window, and
/// announced with a notification the first time it is seen in this run.
pub async fn check(app: &AppHandle) -> Result<Option<String>, String> {
    let latest = fetch_latest().await?;
    let newer = is_newer(&latest, current_version());
    let mut announce = false;
    {
        let state = app
            .try_state::<UpdateState>()
            .ok_or_else(|| "the update check has not started".to_string())?;
        let mut inner = state
            .0
            .lock()
            .map_err(|_| "the update state is poisoned".to_string())?;
        inner.available = newer.then(|| latest.clone());
        if newer && inner.announced.as_deref() != Some(latest.as_str()) {
            inner.announced = Some(latest.clone());
            announce = true;
        }
    }
    crate::tray::update_tray_menu(app);
    if !newer {
        return Ok(None);
    }
    if announce {
        notify(app, &latest);
    }
    if let Err(e) = (UpdateAvailable {
        version: latest.clone(),
    })
    .emit(app)
    {
        log::error!("Failed to emit update-available: {e}");
    }
    Ok(Some(latest))
}

async fn fetch_latest() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(LATEST_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{LATEST_URL} answered {status}"));
    }
    let latest: Latest = response.json().await.map_err(|e| e.to_string())?;
    if parse_version(&latest.version).is_none() {
        return Err(format!("unexpected version {:?}", latest.version));
    }
    Ok(latest.version)
}

/// `major.minor.patch` with an optional leading `v`. Anything else is not a
/// version, so a damaged file can never look like an update.
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.trim().trim_start_matches('v').split('.');
    let mut next = || parts.next()?.parse::<u64>().ok();
    let version = (next()?, next()?, next()?);
    parts.next().is_none().then_some(version)
}

fn is_newer(candidate: &str, current: &str) -> bool {
    matches!(
        (parse_version(candidate), parse_version(current)),
        (Some(candidate), Some(current)) if candidate > current
    )
}

/// A tray string in the app language, or English when the locale lacks it.
fn localized(app: &AppHandle, pick: fn(&TrayStrings) -> &String) -> String {
    let locale = settings::get_settings(app).app_language;
    let text = pick(&get_tray_translations(Some(locale))).clone();
    if text.is_empty() {
        pick(&get_tray_translations(Some("en".to_string()))).clone()
    } else {
        text
    }
}

fn notify(app: &AppHandle, version: &str) {
    let title = localized(app, |s| &s.update_notification_title).replace("{{version}}", version);
    let body = localized(app, |s| &s.update_notification_body);
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        log::warn!("Could not show the update notification: {e}");
    }
}

/// The tray's "Check for Updates..." entry. The outcome is reported in a
/// dialog, because the tray has no other way to answer; a newer version
/// offers to install right away.
pub fn check_from_tray(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match check(&app).await {
            Ok(Some(version)) => offer_install(&app, &version),
            Ok(None) => {
                let message =
                    localized(&app, |s| &s.up_to_date).replace("{{version}}", current_version());
                app.dialog().message(message).title("hushtype").show(|_| {});
            }
            Err(e) => {
                log::warn!("Update check failed: {e}");
                let message = localized(&app, |s| &s.update_check_failed);
                app.dialog().message(message).title("hushtype").show(|_| {});
            }
        }
    });
}

fn offer_install(app: &AppHandle, version: &str) {
    let update = localized(app, |s| &s.update_now);
    let later = localized(app, |s| &s.later);
    let message = format!(
        "{}\n\n{}",
        localized(app, |s| &s.update_notification_title).replace("{{version}}", version),
        localized(app, |s| &s.update_dialog_body)
    );
    let handle = app.clone();
    app.dialog()
        .message(message)
        .title("hushtype")
        .buttons(MessageDialogButtons::OkCancelCustom(update, later))
        .show(move |accepted| {
            if accepted {
                if let Err(e) = run_installer(&handle) {
                    log::error!("Failed to start the updater: {e}");
                }
            }
        });
}

/// Opens Terminal running the published installer, which quits this app,
/// replaces it, resets its Accessibility entry, and reopens it.
pub fn run_installer(app: &AppHandle) -> Result<(), String> {
    let path = std::env::temp_dir().join("Update hushtype.command");
    std::fs::write(&path, installer_script())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("cannot make {} executable: {e}", path.display()))?;
    }
    log::info!("Starting the updater from {}", path.display());
    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<String>)
        .map_err(|e| e.to_string())
}

fn installer_script() -> String {
    format!(
        "#!/bin/bash\n# Written by hushtype to update itself. Safe to delete.\n{INSTALL_COMMAND}\n"
    )
}

/// Manual check from the About settings. Allowed while automatic checks are
/// off; only the environment lock blocks it.
#[tauri::command]
#[specta::specta]
pub async fn check_for_update(app: AppHandle) -> Result<Option<String>, String> {
    if settings::update_checks_forced_disabled() {
        return Err("Update checks are disabled by HUSHTYPE_DISABLE_UPDATER.".into());
    }
    check(&app).await
}

#[tauri::command]
#[specta::specta]
pub fn get_available_update(app: AppHandle) -> Option<String> {
    available_version(&app)
}

#[tauri::command]
#[specta::specta]
pub fn run_update_installer(app: AppHandle) -> Result<(), String> {
    run_installer(&app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_versions_compare_numerically() {
        assert!(is_newer("0.3.0", "0.2.1"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("v1.0.0", "0.9.0"));
        assert!(!is_newer("0.2.1", "0.2.1"));
        assert!(!is_newer("0.2.0", "0.2.1"));
    }

    #[test]
    fn malformed_versions_never_count_as_updates() {
        assert!(!is_newer("", "0.2.1"));
        assert!(!is_newer("0.3", "0.2.1"));
        assert!(!is_newer("0.3.0.1", "0.2.1"));
        assert!(!is_newer("0.3.0-beta", "0.2.1"));
        assert!(!is_newer("0.3.0", "junk"));
    }

    #[test]
    fn installer_script_runs_the_published_command() {
        let script = installer_script();
        assert!(script.starts_with("#!/bin/bash\n"));
        assert!(script.ends_with(&format!("{INSTALL_COMMAND}\n")));
    }
}
