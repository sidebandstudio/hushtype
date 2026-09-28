# hushtype architecture

How a dictation flows through the code, followed by the refine, context, and paste extensions in the order they were built.

## A dictation session

1. `src-tauri/src/shortcut/mod.rs` chooses the native shortcut implementation (`shortcut/native_keys.rs`) or Tauri global shortcuts. Both dispatch into `shortcut/handler.rs::handle_shortcut_event`.
2. `src-tauri/src/transcription_coordinator.rs` serializes press/release events and implements hold, toggle, and hold-or-toggle activation. It calls the actions registered in `actions.rs::ACTION_MAP` and keeps another session from starting while processing is active.
3. `actions.rs::TranscribeAction::start` kicks off model loading and VAD preloading, sets the tray and recording overlay, and calls `managers/audio.rs::AudioRecordingManager::try_start_recording`. Recording readiness is acknowledged only after microphone samples arrive. Cancellation uses a generation counter.
4. `audio_toolkit/audio/recorder.rs` owns the CPAL input stream, ring buffer, recording worker, and VAD policy. `audio/resampler.rs` uses Rubato to produce 16 kHz mono floating-point PCM. `audio_toolkit/vad/` supplies Silero or Earshot VAD and smoothing. The audio manager handles device choice, mute, stream reuse, and stopping.
5. `TranscribeAction::stop` updates the tray, stops capture, and runs the rest on Tauri's async runtime. Empty sample buffers skip transcription. WAV persistence runs in parallel. A live decoder is finalized if present, otherwise `managers/transcription.rs::TranscriptionManager::transcribe` runs batch STT. Model load/unload and exclusive engine ownership live in that manager.
6. `actions.rs::process_transcription_output` performs optional Chinese variant conversion and then refinement (see Phase 1 below). History is saved through `managers/history.rs`, then a main-thread closure calls `utils.rs::paste`, which delegates to `clipboard.rs::paste`. The overlay disappears and the tray returns to idle.

The Rust side owns capture, shortcuts, inference, paste, storage, and orchestration. React does not carry audio or dictate the session state machine. `src-tauri/src/lib.rs` registers Tauri commands through tauri-specta and mounts managers as Tauri state. `src/bindings.ts` contains generated command wrappers and Rust-derived types. Frontend command calls travel through Tauri IPC. Rust emits events for model state, recording levels, streaming text, settings changes, and errors. `src/overlay/RecordingOverlay.tsx` renders the overlay; `src/App.tsx` handles onboarding, settings, and event notifications.

## STT engines and models

`managers/model.rs::EngineType` and `managers/transcription.rs::LoadedEngine` cover:

| Engine             | Loading and inference                                                                                                                                                                                                                        |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| TranscribeCpp      | `transcribe-cpp` 0.2.3 sessions for Whisper-family GGML/GGUF and supported GGUF architectures. On macOS the crate is statically linked with Metal enabled. Model capabilities govern streaming, translation, languages, and initial prompts. |
| Parakeet           | `transcribe-rs` ONNX Parakeet v2/v3, INT8. Batch inference with segment timestamps.                                                                                                                                                          |
| Moonshine          | ONNX base model.                                                                                                                                                                                                                             |
| MoonshineStreaming | ONNX streaming-family model, currently invoked through the batch trait in the batch path.                                                                                                                                                    |
| SenseVoice         | ONNX INT8, with supported language hints.                                                                                                                                                                                                    |
| GigaAM             | ONNX INT8.                                                                                                                                                                                                                                   |
| Canary             | ONNX INT8, language and translation options.                                                                                                                                                                                                 |
| Cohere             | ONNX INT8, language options.                                                                                                                                                                                                                 |

`src-tauri/Cargo.toml` enables `transcribe-rs`'s ONNX support. The macOS ONNX path does not enable CoreML in this checkout. Parakeet therefore uses the available ONNX CPU path, while Whisper can use Metal. Do not describe Parakeet as Metal-accelerated without measuring a different implementation.

`managers/model.rs::ModelManager` merges a registry of known model folders and files with custom files found in the models directory and GGUF models discovered in the shared Hugging Face cache. Models live in the app data directory's `models/` subdirectory. The app never downloads them: `scripts/setup-models.sh` installs them from their original publishers at pinned revisions and checks every file's SHA-256. GGUF metadata and `managers/model_capabilities.rs` drive capability probing.

`commands/models.rs` exposes selection, deletion, discovery, and load commands. Its download command only returns setup instructions. `src/stores/modelStore.ts`, `components/model-selector/`, and `components/settings/models/` provide the UI. The selected ID is stored as `AppSettings.selected_model`. A warm dictation avoids model loading; a cold one waits for the model-load condition variable. Model unload policy can therefore dominate release-to-paste latency.

The batch STT result already receives language-aware filler removal, normalization, and optional fuzzy custom-word correction in `managers/transcription.rs::post_process_transcription_text`. For Whisper, `settings.custom_words` is joined into `WhisperRunOptions.initial_prompt` instead. The ONNX engines do not accept that prompt. Live streaming currently does not receive a dictionary prompt and uses correction afterward. `audio_toolkit/text.rs` implements these text operations.

## macOS paste

The default clipboard path in `clipboard.rs::paste_via_clipboard` saves text or, when no text exists, an image. It writes the transcript, waits the configured before-delay, sends Cmd+V using Enigo, waits the after-delay, and restores the prior clipboard. Enigo posts native keyboard events, which require Accessibility permission. `input.rs` resolves the physical Command-V key through Carbon's current keyboard layout and holds the modifier conservatively for 100 ms. The default path does not set an accessibility text value directly. The Direct option uses Enigo text input instead of a clipboard transaction.

The optional `reliable_paste` setting selects `paste_tx/macos.rs`. It publishes lazy NSPasteboard data using an Objective-C owner object and treats a request for that data as a read receipt. A worker waits for read activity to settle and schedules restoration on the main thread. A pasteboard change-count guard avoids overwriting a newer copy. Its `Ok` return means the transaction started, not that text insertion was confirmed. A clipboard manager can also read the data; a receipt is not proof that the target inserted it. The current timeout restores the old clipboard even if no receipt arrives.

Known failure modes of these paths:

- Under load, earlier clipboard text can be pasted when the target reads after fixed-delay restoration. The receipt path or a longer after-delay mitigates it.
- A missed modifier chord can type a literal `v`, which is why input timing matters.
- Direct input can duplicate or break text in terminal applications, where clipboard paste works.
- Direct input can produce incorrect characters with non-QWERTY layouts.
- Ad-hoc rebuilds can leave stale Accessibility grants: a visibly enabled grant can refer to an older code identity.

These are known limits, not evidence that each reproduces on a given Mac. Synthetic input can also be blocked by secure input or the receiving application. `secure_input.rs` detects the macOS secure-input condition; the frontend has a warning component.

## Settings

`settings.rs::AppSettings` is the Rust schema. It derives Serde and Specta types, supplies defaults, salvages individually valid fields from damaged stores, and applies migrations. `get_settings` reads the `settings` key of `settings_store.json` through `tauri-plugin-store`. `write_settings` replaces that key; the store plugin autosaves. `portable.rs` resolves the path. On an ordinary macOS install it is under `~/Library/Application Support/<bundle-id>/`.

`commands/mod.rs` and the other command modules expose field-specific updates. `src/stores/settingsStore.ts` loads through `commands.getAppSettings`, applies optimistic UI updates, calls the relevant command, rolls back on failure, and refreshes on backend settings events. `src/hooks/useSettings.ts` wraps the Zustand store. Existing `SettingContainer`, `SettingsGroup`, `Input`, `Textarea`, `ToggleSwitch`, and other UI components should be reused.

## Refine insertion point

The narrow seam is `actions.rs::process_transcription_output`, after STT and before history/paste. Both live completion and batch dictation reach it. `commands/history.rs` also uses it for retries. Keep the audio, shortcut coordinator, model engines, and input injection intact. Capture session context at recording start so focus changes during inference cannot select a different mode. Carry that snapshot into the transform and timings.

The existing `post_process_transcription` talks through `llm_client.rs` and has provider-specific handling, structured-output fallbacks, and Apple Intelligence integration. It is not the requested single generic endpoint contract and has no 2500 ms whole-operation budget or semantic hygiene gate. The new `refine` module will own that contract and its offline tests. The current custom-word setting can remain the shared dictionary source.

## Network and verification baseline

Update checks are disabled and the app contains no model downloader, so the only runtime network request is the refine endpoint the user configures. There is no analytics. The headless `--transcribe-file`, `--list-models`, and `--list-devices` paths allow real engine tests without pretending that file input is a live microphone session. Build and runtime evidence belongs in `PROGRESS.md`.

## hushtype refine extension, Phase 1

`src-tauri/src/actions.rs::process_transcription_output` now calls `refine::run` after optional Chinese script conversion and before history/paste. Both primary shortcuts use the global refine setting. The old provider-specific post-processing path is no longer invoked by dictation. Cancellation still wraps the entire transform and suppresses paste.

`src-tauri/src/refine.rs` is independent of Tauri. It sends OpenAI-compatible messages, negotiates optional parameters through server errors, and wraps the whole operation in one deadline: the configured timeout, scaled up for inputs over 200 characters to at most four times the setting. Invalid output, transport/server errors, and timeouts return raw text. Dictionary replacements run afterwards. `src-tauri/resources/refine.txt` contains the prompt and its example pairs. `commands/refine.rs` uses an edited `refine.txt` in app data when one exists and otherwise reads the bundled file, so prompt updates reach unedited installs. `refine::is_retired_default_prompt` recognizes app-data copies that earlier builds made of their stock prompt.

`AppSettings.refine` and `.replacements` use the same store as the other settings. Existing `custom_words` feeds refine, Whisper batch and streaming initial prompts, and local word correction where a prompt is unsupported. `RefineSettings.tsx` and `DictionarySettings.tsx` use the existing settings components and registered Tauri commands.

`dictation_debug.rs::DictationTimer` records early exits and moves into the main-thread paste closure on success. It retains 20 records and writes a 1 MB rolling JSONL log without transcripts or captured context. Phase 3 extends the macOS asynchronous paste path with a completion callback, so timing ends at settlement rather than dispatch. A read receipt is recorded as `clipboard_read`, which does not claim insertion into a particular app.

`eval/` compiles the production refine module separately. Its local HTTP tests are test-only and its 30 fixtures are evaluation data, never shipped application content.

## Context and modes, Phase 2

`context.rs` captures app identity at recording start through `NSWorkspace` and stores a worker for that session. `context_macos.rs` reads the focused window through native Accessibility, using 20 ms per-call timeouts and a 250 ms traversal budget. Browser capture checks the window's document URL, a web area's URL/document, or an identified address field. It skips page descendants and link targets. Unsupported or denied attributes remain absent. There is no AppleScript or synthetic copy operation.

`modes.rs` defines the persisted mode schema, validates the final Default mode, matches exact bundle IDs or anchored URL globs in saved order, and filters context. `refine.rs` sends that context as `app_context` in the final user message, not in the instructions, so the instructions and examples stay a stable prefix that servers can cache between dictations. Each mode's `Style` is applied by `modes::apply_style` after refinement and before replacements. It changes only capitalization and the closing period, so it also applies to raw fallback. Only the matched mode can enable reading selected text or clipboard for context. Those fields default off. The normal paste path still temporarily saves/restores the clipboard; that backup is never sent to the LLM.

The recording pipeline takes its captured worker at stop and resolves it before refinement. Code bypasses refinement by default. Context stays in memory for that session and is not written to transcription history or timing logs. A history retry uses Default without capturing the settings window. `ModesSettings.tsx` edits order, matchers, refinement, prompts, and context permissions through the existing store.

Native references checked against the installed SDK and [Apple's frontmost-application documentation](https://developer.apple.com/documentation/appkit/nsworkspace/frontmostapplication), [Accessibility attribute API](https://developer.apple.com/documentation/applicationservices/1462085-axuielementcopyattributevalue), and [messaging timeout API](https://developer.apple.com/documentation/applicationservices/1459345-axuielementsetmessagingtimeout).

## Completion and fallback, Phase 3

`audio_toolkit/near_silence.rs` adds a final PCM energy check after the recorder stops. Tiny or near-zero audio skips STT, refinement, and paste. `refine.rs` splits long transcripts at sentence/word boundaries, retries smaller chunks only after explicit context-limit errors, and shares one timeout across all requests. Every chunk must pass output hygiene; any failure returns the complete original transcript. Code mode additionally rejects edits to recognizable literal identifiers, paths, flags, and dotted names.

On macOS, `clipboard::paste` uses the `paste_tx` receipt path. Its `PasteCompletion` callback fires at most once, after the native transaction settles. Missing read receipts keep the transcript on the clipboard and display a nonactivating overlay for five seconds. A newer clipboard owner wins. The callback retains the session finish guard until completion, records paste duration, and releases the guard after timing is finalized. The existing 200 ms receipt quiet period and eight-second no-read timeout are unchanged.

Timing records contain audio duration, individual stage durations, cumulative STT/refine/paste completion times from key release, and total duration. The latency reporter measures the STT target from release through STT completion, including capture stop. Logs deliberately distinguish a read receipt from verified insertion. `tray.rs` updates a separate state label without rebuilding the busy menu for every processing state.

## 0.1.1 setup, identity, and optional host

The requested UI redesign uses the existing React settings components and store. `src/components/Sidebar.tsx` renders horizontal navigation; `src/App.css` and `src/styles/theme.css` supply the ivory/green themes. Original mark and Dock icon sources, with generation prompts, are in `assets/brand/`. The packaging icons and template tray images derive from these assets.

`AccessibilityOnboarding.tsx` checks actual macOS grants every 1.5 seconds and on focus, independently of permission-request clicks. It treats request completion and OS permission status separately. Onboarding exposes native privacy-panel links and a process restart for stale grants. `commands/audio.rs` opens the specific macOS privacy panes through the existing opener plugin. It does not grant permissions or edit TCC records.

The normal client remains self-contained for audio capture, local STT, hotkeys, and paste. Only the generic refine request can go to another Mac. `scripts/setup-refine-host.sh` installs a separate launchd job for an existing Ollama installation on the host's Tailscale interface. `serve-refine-host.sh` starts Ollama, preloads the local model, prevents automatic AC-power sleep while serving, and exits when the child exits so launchd can restart it. None of this host setup runs inside hushtype.

Users install through `website/install.sh`, published at hushtype.vercel.app. It downloads the app from the site and the checked Parakeet files from Hugging Face, installs into Applications, and opens the app, without requiring the source tree. The app itself retains the endpoint-only runtime network policy.

Native paste verification found two further failure paths. `context_macos.rs::focused_control_is_nontext` now reads only the focused Accessibility role under a 200 ms deadline, with up to 150 ms for a cold focused-element query. `paste_tx/macos.rs` sends recognized non-text controls directly to clipboard fallback. Unknown/custom controls retain receipt-based paste. A receipt still cannot identify its reader or prove insertion in an arbitrary custom editor.

`overlay.rs::show_clipboard_fallback` queues display from another thread. Tauri can execute `run_on_main_thread` inline when already on that thread; an inline notice would try to acquire Enigo for cursor positioning while the paste caller holds it. Deferring dispatch lets paste release that lock before positioning the notice. `src-tauri/examples/native_paste_check.rs` exercises the production modules and actual overlay against native recipients without adding a command to the shipped app.
