use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Debug, Default, Serialize, Deserialize, Type)]
pub struct TimingRecord {
    pub timestamp: String,
    pub mode: String,
    pub audio_ms: u64,
    pub capture_stop_ms: u64,
    pub stt_ms: u64,
    pub stt_done_ms: u64,
    pub refine_ms: u64,
    pub refine_done_ms: u64,
    pub paste_ms: u64,
    pub paste_done_ms: u64,
    pub total_ms: u64,
    pub outcome: String,
    pub paste_outcome: String,
}

impl TimingRecord {
    pub fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            mode: "Default".into(),
            paste_outcome: "not_attempted".into(),
            ..Self::default()
        }
    }
}

#[derive(Default)]
pub struct DictationTimings(Mutex<VecDeque<TimingRecord>>);

/// Records early returns as well as successful sessions. Move it with the paste
/// closure so scheduling the main thread does not end the measurement early.
pub struct DictationTimer {
    app: AppHandle,
    start: Instant,
    pub timing: TimingRecord,
}

impl DictationTimer {
    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
    pub fn new(app: AppHandle, start: Instant) -> Self {
        let mut timing = TimingRecord::new();
        timing.outcome = "cancelled".into();
        Self { app, start, timing }
    }
}

impl Drop for DictationTimer {
    fn drop(&mut self) {
        self.timing.total_ms = self.start.elapsed().as_millis() as u64;
        record(&self.app, self.timing.clone());
    }
}

/// The rolling log contains durations and outcomes, never transcript or captured context.
pub fn record(app: &AppHandle, timing: TimingRecord) {
    let state = app.state::<DictationTimings>();
    let mut records = state.0.lock().unwrap_or_else(|e| e.into_inner());
    records.push_back(timing.clone());
    while records.len() > 20 {
        records.pop_front();
    }
    let result = (|| -> std::io::Result<()> {
        let dir = crate::portable::app_log_dir(app).map_err(std::io::Error::other)?;
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("dictation-timings.jsonl");
        if std::fs::metadata(&path).is_ok_and(|m| m.len() > 1_000_000) {
            std::fs::rename(&path, dir.join("dictation-timings.previous.jsonl"))?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(file, "{}", serde_json::to_string(&timing)?)
    })();
    if result.is_err() {
        log::warn!("Could not write dictation timing log");
    }
    drop(records);
    let _ = app.emit("dictation-timing", &timing);
}

#[tauri::command]
#[specta::specta]
pub fn get_dictation_timings(app: AppHandle) -> Vec<TimingRecord> {
    app.state::<DictationTimings>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .rev()
        .cloned()
        .collect()
}
