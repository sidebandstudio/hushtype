//! Prompt comparison with explicit fixture identities, not a live-app demo.
#[allow(dead_code)]
#[path = "../../../src-tauri/src/modes.rs"]
mod modes;
#[allow(dead_code)]
#[path = "../../../src-tauri/src/refine.rs"]
mod refine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = include_str!("../../../src-tauri/resources/refine.txt");
    let raw = "um I am going to send the report tomorrow I will follow up after lunch";
    let settings = refine::RefineSettings::default();
    let mut warm = settings.clone();
    warm.timeout_ms = 120_000;
    let _ = refine::run(&warm, "hello", base, &[]).await;
    let mut rows = Vec::new();
    for bundle in [
        "com.tinyspeck.slackmacgap",
        "com.apple.mail",
        "com.todesktop.230313mzl4w4u92",
    ] {
        let context = modes::CapturedContext {
            bundle_id: Some(bundle.into()),
            ..Default::default()
        };
        let mode = modes::match_mode(&modes::default_modes(), &context);
        let resolved = modes::ResolvedContext { mode, context };
        let mut config = settings.clone();
        config.enabled = resolved.mode.refine_enabled;
        let result = refine::run(&config, raw, &resolved.prompt(base), &[]).await;
        rows.push(serde_json::json!({"fixture_bundle":bundle,"mode":resolved.mode.name,"raw":raw,"result":result}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"kind":"text-only mode comparison; explicit fixture identities, not captured apps", "results":rows})
        )?
    );
    Ok(())
}
