use crate::refine::{RefineSettings, Replacement};
use crate::settings::{get_settings, write_settings};
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;

pub fn prompt_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::portable::app_data_dir(app).map_err(|_| "Cannot locate app data.")?;
    std::fs::create_dir_all(&dir).map_err(|_| "Cannot create app data directory.")?;
    let path = dir.join("refine.txt");
    if path.exists() {
        return Ok(path);
    }
    let bundled = app
        .path()
        .resolve("resources/refine.txt", tauri::path::BaseDirectory::Resource)
        .map_err(|_| "Cannot locate the bundled refine prompt.")?;
    let contents = std::fs::read(bundled).map_err(|_| "Cannot read the bundled refine prompt.")?;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            file.write_all(&contents)
                .map_err(|_| "Cannot write the refine prompt.")?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err("Cannot create the refine prompt.".into()),
    }
    Ok(path)
}

#[tauri::command]
#[specta::specta]
pub fn get_refine_prompt(app: AppHandle) -> Result<String, String> {
    let path = prompt_path(&app)?;
    if std::fs::metadata(&path)
        .map_err(|_| "Cannot read prompt metadata.")?
        .len()
        > 65_536
    {
        return Err("The refine prompt must be smaller than 64 KiB.".into());
    }
    std::fs::read_to_string(path).map_err(|_| "Cannot read the refine prompt as UTF-8.".into())
}

#[tauri::command]
#[specta::specta]
pub fn set_refine_prompt(app: AppHandle, prompt: String) -> Result<(), String> {
    if prompt.trim().is_empty() || prompt.len() > 65_536 {
        return Err("The prompt must contain text and be smaller than 64 KiB.".into());
    }
    let path = prompt_path(&app)?;
    let temp = path.with_extension("txt.new");
    std::fs::write(&temp, prompt).map_err(|_| "Cannot save the refine prompt.")?;
    std::fs::rename(temp, path).map_err(|_| "Cannot replace the refine prompt.".into())
}

#[tauri::command]
#[specta::specta]
pub fn update_refine_settings(app: AppHandle, refine: RefineSettings) -> Result<(), String> {
    refine.validate()?;
    let mut settings = get_settings(&app);
    settings.refine = refine;
    write_settings(&app, settings);
    save(&app)
}

#[tauri::command]
#[specta::specta]
pub fn update_replacements(app: AppHandle, replacements: Vec<Replacement>) -> Result<(), String> {
    if replacements.len() > 1000
        || replacements
            .iter()
            .any(|r| r.from.trim().is_empty() || r.from.len() > 200 || r.to.len() > 200)
    {
        return Err("Use at most 1000 replacements, with a nonempty source and at most 200 bytes per field.".into());
    }
    let mut settings = get_settings(&app);
    settings.replacements = replacements;
    write_settings(&app, settings);
    save(&app)
}

pub fn save(app: &AppHandle) -> Result<(), String> {
    app.store(crate::portable::store_path(
        crate::settings::SETTINGS_STORE_PATH,
    ))
    .map_err(|_| "Cannot open settings.")?
    .save()
    .map_err(|_| "Cannot save settings.")?;
    let _ = app.emit("settings-changed", ());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn update_modes(app: AppHandle, modes: Vec<crate::modes::Mode>) -> Result<(), String> {
    crate::modes::validate_modes(&modes)?;
    let mut settings = get_settings(&app);
    settings.modes = modes;
    write_settings(&app, settings);
    save(&app)
}
