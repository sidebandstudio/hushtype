#[path = "../../src-tauri/src/refine.rs"]
mod refine;

use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

#[derive(Deserialize)]
struct Fixture {
    id: String,
    raw: String,
    expected: String,
    #[serde(default)]
    dictionary: Vec<String>,
}

#[derive(Serialize)]
struct ResultRow {
    id: String,
    raw: String,
    expected: String,
    actual: String,
    outcome: String,
    duration_ms: u64,
    passed: bool,
}

fn normalize(text: &str) -> String {
    text.replace('’', "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let app_data = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join("Library/Application Support/dev.hushtype.app");
    let stored = fs::read(app_data.join("settings_store.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
    let mut settings: refine::RefineSettings = stored
        .as_ref()
        .and_then(|s| s.pointer("/settings/refine"))
        .cloned()
        .and_then(|s| serde_json::from_value(s).ok())
        .unwrap_or_default();
    settings.enabled = true;
    if let Ok(value) = env::var("HUSHTYPE_REFINE_BASE_URL") {
        settings.base_url = value;
    }
    if let Ok(value) = env::var("HUSHTYPE_REFINE_MODEL") {
        settings.model = value;
    }
    if let Ok(value) = env::var("HUSHTYPE_REFINE_API_KEY") {
        settings.api_key = value;
    }
    if let Ok(value) = env::var("HUSHTYPE_REFINE_TIMEOUT_MS") {
        settings.timeout_ms = value.parse()?;
    }
    settings.validate()?;
    let prompt_path = env::var_os("HUSHTYPE_REFINE_PROMPT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let path = app_data.join("refine.txt");
            if path.exists() {
                path
            } else {
                dir.join("../src-tauri/resources/refine.txt")
            }
        });
    let prompt = fs::read_to_string(prompt_path)?;
    let fixtures: Vec<Fixture> =
        serde_json::from_slice(&fs::read(dir.join("fixtures/cleanup.json"))?)?;
    // Loading a local model is not a warm inference measurement. The actual
    // fixtures below still use the configured whole-stage timeout.
    if !env::args().any(|a| a == "--no-warmup") {
        let mut warm = settings.clone();
        warm.timeout_ms = 120_000;
        let result = refine::run(&warm, "the release is ready", &prompt, &[]).await;
        println!(
            "Warmup: {} ({} ms, excluded from score/timings)",
            result.outcome, result.duration_ms
        );
    }
    let mut results = Vec::new();
    for fixture in fixtures {
        let result = refine::run(&settings, &fixture.raw, &prompt, &fixture.dictionary).await;
        let passed =
            result.outcome == "cleaned" && normalize(&result.text) == normalize(&fixture.expected);
        println!(
            "{} {} ({} ms, {})",
            if passed { "PASS" } else { "FAIL" },
            fixture.id,
            result.duration_ms,
            result.outcome
        );
        if !passed {
            println!("- {}\n+ {}", fixture.expected, result.text);
        }
        results.push(ResultRow {
            id: fixture.id,
            raw: fixture.raw,
            expected: fixture.expected,
            actual: result.text,
            outcome: result.outcome,
            duration_ms: result.duration_ms,
            passed,
        });
    }
    let passed = results.iter().filter(|r| r.passed).count();
    let mut times: Vec<_> = results.iter().map(|r| r.duration_ms).collect();
    times.sort_unstable();
    let report = serde_json::json!({
        "model": settings.model, "timeout_ms": settings.timeout_ms,
        "passed": passed, "total": results.len(),
        "p50_ms": times[(times.len()-1)/2], "p95_ms": times[(times.len()*95).div_ceil(100)-1],
        "results": results,
    });
    fs::write(
        dir.join("latest.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    println!("{passed}/{} passed. Results: eval/latest.json", times.len());
    if passed < 27 {
        std::process::exit(1);
    }
    Ok(())
}
