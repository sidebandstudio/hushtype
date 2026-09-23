// Developer-only executable. Include the production modules so this check
// exercises the same native paste transaction and keyboard injection code.
// Cargo does not include examples in the shipped application bundle.
include!("../src/lib.rs");

#[cfg(target_os = "macos")]
fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() != 3 {
        eprintln!("Usage: native_paste_check <frontmost-bundle-id> <test-text>");
        std::process::exit(2);
    }
    let expected_bundle = arguments[1].clone();
    let text = arguments[2].clone();
    let mut context = tauri::generate_context!(test = true);
    context.config_mut().identifier = "dev.hushtype.paste-verification".into();
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_nspanel::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Debug)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![commands::get_app_settings])
        .setup(move |app| {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            app.manage(input::EnigoState::new()?);
            overlay::create_recording_overlay(app.handle());
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                // Let the native event loop start without taking focus.
                std::thread::sleep(std::time::Duration::from_secs(2));
                let app = handle.clone();
                let _ = handle.run_on_main_thread(move || {
                    unsafe extern "C" {
                        fn AXIsProcessTrusted() -> bool;
                    }
                    if !unsafe { AXIsProcessTrusted() } {
                        eprintln!("Native check needs Accessibility access; no paste attempted.");
                        app.exit(2);
                        return;
                    }
                    let actual = objc2::rc::autoreleasepool(|_| {
                        objc2_app_kit::NSWorkspace::sharedWorkspace()
                            .frontmostApplication()
                            .and_then(|front| front.bundleIdentifier())
                            .map(|bundle| bundle.to_string())
                    });
                    if actual.as_deref() != Some(expected_bundle.as_str()) {
                        eprintln!("Target lost focus; no paste attempted.");
                        app.exit(2);
                        return;
                    }
                    let finished = app.clone();
                    let completion = paste_tx::PasteCompletion::new(move |outcome| {
                        println!("paste_outcome={outcome}");
                        std::thread::spawn(move || {
                            // Keep the real fallback overlay alive for inspection.
                            std::thread::sleep(std::time::Duration::from_secs(6));
                            finished.exit(0);
                        });
                    });
                    let state = app.state::<input::EnigoState>();
                    let result = state
                        .0
                        .lock()
                        .map_err(|e| e.to_string())
                        .and_then(|mut enigo| {
                            paste_tx::try_reliable_paste(
                                &text,
                                &app,
                                &settings::PasteMethod::CtrlV,
                                &mut enigo,
                                false,
                                settings::AutoSubmitKey::Enter,
                                settings::ClipboardHandling::DontModify,
                                Some(completion),
                            )
                        });
                    if let Err(error) = result {
                        eprintln!("Paste failed: {error}");
                        app.exit(1);
                    }
                });
            });
            Ok(())
        })
        .run(context)
        .expect("Could not start native paste verification");
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Native paste verification requires macOS.");
    std::process::exit(2);
}
