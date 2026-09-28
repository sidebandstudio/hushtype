#[allow(dead_code)]
#[path = "../../src-tauri/src/modes.rs"]
mod modes;
#[path = "../../src-tauri/src/refine.rs"]
mod refine;

use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

#[derive(Deserialize)]
struct Fixture {
    id: String,
    raw: String,
    /// Exact output, ignoring whitespace and apostrophe style.
    #[serde(default)]
    expected: Option<String>,
    /// For long real dictations: phrases that must, or must not, appear.
    #[serde(default)]
    contains: Vec<String>,
    #[serde(default)]
    absent: Vec<String>,
    /// A shipped mode whose prompt addendum applies, as in the app.
    #[serde(default)]
    mode: Option<String>,
    /// App context as the app sends it: untrusted data in the final message.
    #[serde(default)]
    context: Option<serde_json::Value>,
    #[serde(default)]
    dictionary: Vec<String>,
}

#[derive(Serialize)]
struct ResultRow {
    id: String,
    suite: String,
    raw: String,
    expected: String,
    actual: String,
    outcome: String,
    duration_ms: u64,
    passed: bool,
}

const SUITES: &[&str] = &["cleanup", "disfluency"];

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
    // Same choice as the app: an edited app-data prompt, else the bundled one.
    let prompt = match env::var_os("HUSHTYPE_REFINE_PROMPT") {
        Some(path) => fs::read_to_string(path)?,
        None => fs::read_to_string(app_data.join("refine.txt"))
            .ok()
            .filter(|p| !refine::is_retired_default_prompt(p))
            .map_or_else(
                || fs::read_to_string(dir.join("../src-tauri/resources/refine.txt")),
                Ok,
            )?,
    };
    // `--suite NAME` runs one suite, for quick prompt iteration.
    let only = env::args().skip_while(|a| a != "--suite").nth(1);
    let suites: Vec<&str> = SUITES
        .iter()
        .copied()
        .filter(|s| only.as_deref().is_none_or(|o| o == *s))
        .collect();
    let mut fixtures = Vec::new();
    for suite in &suites {
        let path = dir.join(format!("fixtures/{suite}.json"));
        for fixture in serde_json::from_slice::<Vec<Fixture>>(&fs::read(path)?)? {
            fixtures.push((*suite, fixture));
        }
    }
    let modes = modes::default_modes();
    // Loading a local model is not a warm inference measurement. The actual
    // fixtures below still use the configured whole-stage timeout.
    if !env::args().any(|a| a == "--no-warmup") {
        let mut warm = settings.clone();
        warm.timeout_ms = 120_000;
        let result = refine::run(&warm, "the release is ready", &prompt, &[], None).await;
        println!(
            "Warmup: {} ({} ms, excluded from score/timings)",
            result.outcome, result.duration_ms
        );
    }
    let mut results = Vec::new();
    for (suite, fixture) in fixtures {
        let prompt = match &fixture.mode {
            Some(id) => modes::ResolvedContext {
                mode: modes
                    .iter()
                    .find(|m| &m.id == id)
                    .ok_or("unknown mode")?
                    .clone(),
                context: Default::default(),
            }
            .prompt(&prompt),
            None => prompt.clone(),
        };
        let result = refine::run(
            &settings,
            &fixture.raw,
            &prompt,
            &fixture.dictionary,
            fixture.context.as_ref(),
        )
        .await;
        let actual = normalize(&result.text);
        let passed = result.outcome == "cleaned"
            && fixture
                .expected
                .as_ref()
                .is_none_or(|e| normalize(e) == actual)
            && fixture
                .contains
                .iter()
                .all(|c| actual.contains(&normalize(c)))
            && !fixture
                .absent
                .iter()
                .any(|a| actual.contains(&normalize(a)));
        println!(
            "{} {} ({} ms, {})",
            if passed { "PASS" } else { "FAIL" },
            fixture.id,
            result.duration_ms,
            result.outcome
        );
        let expected = fixture.expected.unwrap_or_else(|| {
            format!(
                "contains {:?}, absent {:?}",
                fixture.contains, fixture.absent
            )
        });
        if !passed {
            println!("- {}\n+ {}", expected, result.text);
        }
        results.push(ResultRow {
            id: fixture.id,
            suite: suite.into(),
            raw: fixture.raw,
            expected,
            actual: result.text,
            outcome: result.outcome,
            duration_ms: result.duration_ms,
            passed,
        });
    }
    let passed = results.iter().filter(|r| r.passed).count();
    let mut failing_suite = false;
    let mut summary = serde_json::Map::new();
    for suite in &suites {
        let rows: Vec<_> = results.iter().filter(|r| r.suite == *suite).collect();
        let suite_passed = rows.iter().filter(|r| r.passed).count();
        println!("{suite}: {suite_passed}/{}", rows.len());
        // 27 of 30 was the original bar; every suite keeps the same 90%.
        failing_suite |= suite_passed * 10 < rows.len() * 9;
        summary.insert(
            suite.to_string(),
            serde_json::json!({"passed": suite_passed, "total": rows.len()}),
        );
    }
    let mut times: Vec<_> = results.iter().map(|r| r.duration_ms).collect();
    times.sort_unstable();
    let report = serde_json::json!({
        "model": settings.model, "timeout_ms": settings.timeout_ms,
        "passed": passed, "total": results.len(), "suites": summary,
        "p50_ms": times[(times.len()-1)/2], "p95_ms": times[(times.len()*95).div_ceil(100)-1],
        "results": results,
    });
    fs::write(
        dir.join("latest.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    println!("{passed}/{} passed. Results: eval/latest.json", times.len());
    if failing_suite {
        std::process::exit(1);
    }
    Ok(())
}
