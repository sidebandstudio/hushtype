# Progress

## 2026-09-21, Phase 0 inspection

- Resumed after interruption. `/Users/aldo/hushtype` was empty and was not a Git repository. No prior build processes were running.
- Cloned the base engine source attributed in `THIRD_PARTY_NOTICES` and recorded its base commit.
- Read repository instructions, build guidance, license, session actions, shortcut handler/coordinator, audio capture, transcription engines, model registry, settings/store flow, clipboard/input code, native paste receipts, app setup, and About/sidebar code.
- Read known paste failure reports and documented them in `ARCHITECTURE.md`. Wrote architecture and initial decisions before any source edits.
- Host checks: macOS 15.7.9, arm64; Rust/Cargo 1.98.0; Node 26.0.0; approximately 656 GiB free. Xcode is installed, active developer directory is CommandLineTools. Bun, CMake, Ollama, and the Cargo Tauri command were absent from PATH.
- No build, launch, microphone dictation, paste demo, eval, or latency check has passed yet.

## Acceptance evidence

| Requirement                                           | Status                                             |
| ----------------------------------------------------- | -------------------------------------------------- |
| Signed Apple Silicon app build                        | Passed through Phase 3, ad-hoc signature verified  |
| Launch on clean macOS 14+ machine                     | Pending                                            |
| hushtype UI identity and license notices              | Static rename/notices verified; visual UI pending  |
| Local Ollama eval at least 27/30                      | Passed, 27/30 in Phase 1                           |
| Refine failure falls back to raw paste without dialog | Rust client/outage test passed; live paste pending |
| Messaging / Email / Code demo                         | Text-only comparison passed; live apps pending     |
| 20 real dictations and latency report                 | Pending                                            |
| Architecture, decisions, and progress kept current    | In progress                                        |

## 2026-09-22, Phase 0 build in progress

- Installed the missing build tools and local Ollama. Started Ollama on loopback with cloud access disabled. Pulled the requested `qwen3:8b` model. Downloads were still in progress at this entry.
- Renamed the bundle, window, tray text, wordmark, and locale copy to hushtype. Added `LICENSE.hushtype` and bundled both notices. Preserved the original README separately.
- Disabled the updater and its frontend entry points, hid uninstalled models, and disabled the download IPC operation. Added `scripts/setup-models.sh` for an external, checksum-verified Parakeet V3 installation.
- Ran `cargo tauri build`. The first attempt exposed an empty ErrorBoundary after removal of What's New; removed it. The second passed TypeScript and Vite and began compiling the native dependencies.
- Frontend ESLint passed. A host probe reported Accessibility false and Microphone not determined for the command runner. These are not grants for the new app; app permissions must be checked after launch.
- Created a 16 kHz mono WAV using macOS Samantha TTS for a file-based STT smoke check. This is synthetic test audio and does not count toward the real-dictation acceptance requirement.

### Phase 0 checks performed

- `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo tauri build` produced an ad-hoc signed arm64 app. `codesign --verify --deep --strict` passed. The bundle ID is `dev.hushtype.app`; both packaged license files compare byte for byte with their source files.
- The base engine's `LICENSE` is unchanged, SHA-256 `3564e8ec14055ed54f896e8660a62627ea4fc3a1b64f72b798593df00e1f4edc`.
- Ran the bundled executable against the synthetic 3.718-second WAV using `--transcribe-file`, Parakeet V3, three repeats, and JSON output. Result: "Send the project update on Wednesday, the build is ready for review." Model load 1253 ms; STT 776, 775, 694 ms. Build activity was concurrent, so this is a smoke check, not a latency benchmark.
- Ran `cargo clippy --release`: completed with warnings in inherited code and the now-unreachable downloader, no errors. Removed the unused import introduced by disabling updater checks. ESLint, changed-file Prettier checks, Cargo formatting, and `git diff --check` passed.
- Launched the app with Launch Services. The native process and tray initialized, but WebKit reported `web content process terminated`. No hushtype windows appeared in the window list. IORegistry confirms `CGSSessionScreenIsLocked=Yes`; loginwindow holds Secure Input. Asked Ali to unlock the session for GUI checks. No hotkey, live dictation, visual UI, or paste check is claimed.
- The second `cargo tauri build` passed with the final source and UI changes. Rechecked its ad-hoc signature successfully. Build output is `/Users/aldo/hushtype/src-tauri/target/release/bundle/macos/hushtype.app`.
- Ollama is running locally with cloud disabled and `qwen3:8b` installed. A `/v1/chat/completions` smoke request with `reasoning_effort: none` returned "Ready." Sending only a no-thinking instruction in the prompt still generated reasoning and exhausted the small completion limit. This will inform the generic client's optional-parameter handling.

The Phase 0 implementation is built, signed, and exercises the original local STT path. The live checkpoint remains pending desktop access. Continue independent implementation after this code checkpoint; do not count deferred manual checks as passes.

## 2026-09-22, Phase 1 integration

- Added the generic Rust refine client, editable prompt, output guard, replacements, and rolling timing records. Connected refinement to the existing post-STT seam and the default shortcut. Audio capture and hotkey implementations remain unchanged.
- Added 30 original, fixed eval fixtures and a runner that compiles the production module. The first local Ollama run passed 26/30; self-corrections and adversarial dictation need prompt work. Later trials scored 26/30 and 25/30, so no eval pass is claimed yet.
- Eleven socket-based fallback/hygiene tests passed, including a whole-request deadline and exact raw text preservation. The first integrated cargo check found a string error type mismatch; fixed it.
- Added Refine and Dictionary settings using the existing UI components and store, with last-20 timing display in Refine. Extending Whisper streaming options to carry the same custom words as batch transcription.
- A debug build is running to generate bindings from registered Rust commands before the production app build. GUI and microphone checks remain gated by the locked desktop.

### Phase 1 verification so far

- The final prompt trial passed 27/30 on local `qwen3:8b`, default 2500 ms deadline. Warm refine p50 300 ms, p95 489 ms. Results are committed under `eval/results/`; these are text-only timings. The failed date correction is now rejected to raw instead of accepting the abandoned date. The other two strict misses are comma/capitalization differences. Expected outputs were not changed.
- Thirteen refine tests passed. The integrated debug build completed and generated TypeScript bindings. Frontend TypeScript/Vite and ESLint passed. Release Clippy passed with inherited and disconnected legacy-code warnings.
- Terminated the Ollama server owned by this session after the first fixture during a real eval run. All 29 subsequent requests returned exact raw text. Recorded `eval/results/phase1-ollama-outage.json`, then restarted Ollama. This tests the real Rust client against endpoint loss, not clipboard behavior or absence of a GUI dialog.
- The desktop remains locked according to IORegistry. Production build and action cancellation tests are still running at this entry.

- The production `cargo tauri build` completed. Ad-hoc signature verification passed; the bundled original license and refine prompt match their sources. The main executable is arm64.
- Three action cancellation/overlay tests and 24 settings parsing/migration tests passed. Changed-file formatting checks, Cargo formatting, and `git diff --check` passed.

- Ran the newly bundled executable on the same synthetic WAV used for Phase 0. Parakeet returned: "Send the project update on Wednesday, the build is ready for review.". This confirms the packaged STT path still runs; it is not a real microphone or paste check. Phase 1 implementation checkpoint is ready, with live dictation/UI checks explicitly pending.

## 2026-09-22, Phase 2 integration

- Implemented native frontmost-app and bounded Accessibility capture, ordered modes, per-mode prompt/context controls, and the Modes settings section. Selected text and clipboard are opt-in; context is not persisted with history. No new dependencies or AppleScript.
- Four mode tests passed, covering app/URL matching, first-match priority, Default validation, prompt placement, and optional-reader privacy. Ran a native context test against the actual session. It returned `com.apple.loginwindow`, Default, no accessible window or URL. This confirms the FFI runs, not that browser URL capture works in an unlocked app.
- The first text-only mode comparison produced identical Messaging and Email wording. Made the Messaging addendum explicit about natural contractions, then reran. The committed comparison now shows distinct Messaging, Email, and Code results in `eval/results/phase2-mode-comparison.json`. These use fixture bundle IDs and do not count as the real three-app demo.
- TypeScript/Vite, ESLint, and release Clippy passed. A production Tauri build is running. GUI capture, settings interactions, and paste remain pending desktop access.

- The Phase 2 `cargo tauri build` completed. `codesign --verify --deep --strict` passed on the new app. Invoked Launch Services for the built app after terminating the older test instance; the first launch returned error -600. The desktop is still locked. This does not prove a usable webview or successful dictation.
- Cargo formatting, changed-file Prettier, and `git diff --check` passed. Phase 2 implementation checkpoint is ready; the live three-app and browser checks remain pending.

## 2026-09-22, Phase 3 integration

- Added the post-capture near-silence gate, bounded long-input chunks with one shared deadline, smaller-chunk retry for explicit context-limit errors, and complete raw fallback on any chunk failure. Added a literal-token guard for Code mode when refinement is enabled.
- Extended the existing macOS paste receipt transaction. It keeps plain text on the clipboard when no read arrives, respects newer clipboard copies, and reports completion through a once-only callback. Timing now ends at settlement, before optional model unload. A short nonactivating overlay announces clipboard fallback. The menu bar now distinguishes Idle, Listening, Transcribing, and Refining.
- The Phase 2 Launch Services retry succeeded after the prior process exited. Its locked-session webview behavior is still not a usable GUI check.
- The full Rust suite passed 293 tests before the final literal-token additions. The independent refine/mode crate now passes 22 tests, including whole-operation chunk timeout and exact raw fallback after a partial chunk succeeds. TypeScript/Vite and ESLint passed. Release Clippy passed with existing warnings and two new argument-count warnings from adding paste completion without restructuring the existing paste API.
- The local Ollama regression eval still passes 27/30. Saved `eval/results/phase3-qwen3-8b.json`. No real dictation timing records exist on this locked host; `verification/phase3-real-latency.json` records zero completed attempts and leaves acceptance unmeasured. Added `scripts/report-latency.py` for the actual 20-session sample, including raw fallback attempts.
- Extended the external setup script to verify/install the VAD resource as well as Parakeet, and ran it successfully against the existing verified assets. Wrote setup, endpoints, modes, dictionary, privacy, fallback, and debugging instructions in README.
- Generated a 9.475-second Samantha TTS WAV for a separate packaged STT diagnostic. This is synthetic audio and does not substitute for any of the required 20 real dictations. The first Phase 3 release build is still running; a final build will include the last hygiene fixes.

### Phase 3 final verification

- The final `cargo tauri build` completed. Rechecked the ad-hoc signature, `dev.hushtype.app` identifier, and both packaged MIT notices. The original license SHA-256 remains unchanged. The executable targets macOS 14.0 and links only system frameworks/libraries; there are no devbox-specific dynamic library paths.
- The final full Rust suite passed 296 tests. The isolated eval crate passed 22 tests in its mode-comparison binary, including the production refine tests. ESLint, TypeScript/Vite, changed-file Prettier, Cargo formatting, and `git diff --check` passed. Release Clippy completed with documented warnings.
- Ran the final packaged executable on one 9.475-second generated WAV, repeated 20 times. Synthetic Parakeet inference p50 372 ms, p95 412 ms, maximum 432 ms; model load 772 ms. Results and method are in `verification/phase3-synthetic-stt.*`. This supports engine-speed diagnosis only. It does not measure capture-stop overhead or any real end-to-end dictation.
- Reran the final production refine module against local Ollama: 27/30 passed, warm text-only p50 303 ms and p95 544 ms. Saved the final report in `eval/results/phase3-qwen3-8b.json`.
- Packaged the signed app as `artifacts/hushtype-0.1.0-apple-silicon.zip`, 18,096,610 bytes. Extracted the archive into a fresh temporary directory and verified the app signature again. Verified HTTP download access on devbox1 port 8842. The artifact server exposes only the artifacts directory and does not change T3 Code or Tailscale Serve.
- Invoked Launch Services on the final build. IORegistry still reports a locked session, so this is not a successful visual UI or live-dictation check. Asked which Mac can provide an unlocked desktop and a live microphone for the remaining checks.

### Remaining acceptance gate

There are zero real dictations in the timing log. The required 20-session latency report is unmeasured, not passed. Live hotkey/microphone/paste checks, endpoint loss with real paste, browser URL capture, the Slack/Mail/Cursor demo, and clean-machine launch remain pending. The source implementation and automated checks are complete for phases 0–3. Stretch work has not started.

## 2026-09-23, unlocked desktop and requested redesign

- Verified that devbox1 is unlocked. Restarted the packaged app and captured its real permissions screen at `/tmp/hushtype-unlocked-window.png`; the webview renders. The earlier WebKit termination log does not by itself prove a blank main window.
- The first visual check found that the wordmark renders the missing `hushtype.name` translation key. Several other hushtype identity/setup translations are also absent from the final English dictionary. These must be corrected before UI acceptance.
- Ali requested an original logo and a distinct UI. Began that work and recorded the design choices before changing the UI.
- Native screenshot, Accessibility inspection, and input automation now work through the temporary test driver after permission was enabled. The app itself still showed Microphone and Accessibility onboarding at the last inspection. Live human dictation and clean-machine acceptance are still pending.
- The first redesigned 0.1.1 production build passed on devbox1, and its ad-hoc signature verifies. Installed this bundle at `/Users/aldo/Applications/hushtype.app`. Release Clippy completed with warnings; TypeScript/Vite and ESLint passed.
- Two permission recovery regression tests pass in Chrome: external grants are detected without first clicking a request button, and a completed request is never mistaken for an actual grant. Native verification still shows an old Accessibility entry enabled while the newly installed process reports denied access. A toggle and restart did not resolve it; testing a normal remove/add of the exact installed app next.
- Ali deferred the 20 live microphone dictations to his MacBook. Continue UI, setup, context, paste, and server reliability checks here. No synthetic recording will be counted toward that human sample.
- Native permission recovery succeeded: `tccutil reset Accessibility dev.hushtype.app` reset only hushtype’s stale record, then a normal grant in System Settings let the running 0.1.1 app advance automatically to model selection. Microphone is also granted. No TCC database edits or permission bypasses were used. The old entry was not reliable evidence of access for the current ad-hoc binary.
- Installed `dev.hushtype.ollama` as a user launch agent bound to `100.80.149.7:11434`, replacing the temporary Ollama process owned by this session. Its listener and launchd state are verified; preload and restart testing are in progress.
- Selected installed Parakeet V3 through native onboarding. The settings screen rendered with the new logo, navigation, and palette; saved `/tmp/hushtype-dictation-dark.png`.
- Native UI checks passed for changing/saving the endpoint, adding/editing/reordering a temporary mode, deleting it back to the four defaults, and adding the requested `open claw` → `OpenClaw` replacement. Enabled Start Hidden and Launch on Startup; the native log confirms successful SMAppService registration.
- The initial host service run failed all 30 eval fixtures by timeout (saved in `eval/results/host-initial-background.json`). It allocated a 40,960-token context and ran under launchd background throttling. Changed the host to a 4096-token context and Interactive process priority. The same fixtures then passed 27/30, p50 302 ms, p95 541 ms (`eval/results/host-tailscale-qwen3-8b.json`). These are text-only requests from devbox1 to its Tailscale interface, not MacBook network latency or live dictation.
- Terminated the service-owned Ollama process to exercise recovery. launchd restarted it with a new PID, and `/api/ps` confirms qwen3:8b automatically reloaded with the 4096-token context and indefinite retention. Only the Tailscale address listens on port 11434. Fixed a setup-script race by retrying bootstrap while the previous launch job finishes exiting.
- Native Accessibility context checks now assert the exact opened URL for Safari, Google Chrome, and Brave. All three passed without selected-text or clipboard reads. Mail matched Email, Cursor matched Code, and Messages matched Messaging. The first Brave/Cursor attempts detected a different frontmost app and correctly failed; reruns explicitly activated the intended app. The Messages probe initially used the obsolete iChat bundle ID; the production default already uses the correct `com.apple.MobileSMS`, which passed. Arc is not installed here. These are context checks, not the live three-app dictation demo.
- Both light and dark native views render. Visual inspection found the sticky Save row clipped by the footer; corrected its bottom offset before the final build. Added a separate double-clickable speech-model installer for the app archive, with no in-app network download and an explicit failure for incomplete existing model folders. A fresh download/hash/extraction check is running.
- A developer-only Cargo example includes the production Rust modules and calls the actual macOS paste transaction, with a guard against pasting if the expected app loses focus. Into a new temporary TextEdit document it reported `clipboard_read`; native AX inspection confirmed the exact test sentence was inserted. This verifies native injection and receipt settlement, separately from microphone dictation. The example is not shipped in the app.
- The clean external model installation passed a real 456 MB download, the pinned archive SHA-256 check, and extraction into `/tmp/hushtype-model-install-verification`. ESLint passed, and both permission regression tests passed again after final frontend edits.
- The native blocked-input check did not pass: even a separate native window that ignores keyboard input resulted in `clipboard_read`, and the old clipboard was restored. A consumer outside the target can read the lazy clipboard promise. This confirms the documented receipt limitation in practice. TextEdit insertion is verified, but arbitrary blocked-input detection and its notice remain unverified; the receipt outcome must not be described as proof of insertion. No workaround that reads private target text or changes unrelated clipboard software was applied.
- The final 0.1.1 `cargo tauri build` and strict ad-hoc signature verification passed. Release Clippy completed with the existing 53 warnings. Installed the exact bundle at `/Users/aldo/Applications/hushtype.app`, retaining the previous installed build in a temporary backup. macOS requires renewed grants for this new ad-hoc code identity; performing them through the normal dialogs.
- Following the failed blocked-input test, added a focused-role preflight to the existing paste transaction. Native inspection identified the rejecting test target as `AXWindow`. Known non-text roles now copy the transcript and request the manual-paste notice immediately, before synthetic input. Unknown/custom controls keep the existing receipt path. Rebuilding the native check before repeating TextEdit and the rejecting-window cases; the preceding installed build is superseded by this fix.
- The focused-role correction passed both native cases: TextEdit inserted the exact sentence again, and the rejecting window now returns `clipboard_fallback` with the full plain text surviving process exit.
- Adding the real recording overlay to the native check exposed a deadlock: the fallback notice’s inline main-thread positioning tried to lock Enigo while paste already held it. Captured the native stack, then changed notice dispatch to queue from another thread so the current paste releases its lock first. The overlay check is being rerun before the production build is accepted.
- Checked the installed UI at the 760 px minimum width: navigation fits and the Save button is fully visible. History and Models render correctly. A real capture/stop using the closed-lid built-in device produced zero samples, logged `empty_audio`/`not_attempted`, and made no STT, refine, or paste attempt. This is a silence check, not a human dictation.
- The actual native clipboard notice now renders: `/tmp/hushtype-clipboard-notice.png` shows “Text copied. Press Command-V to paste.” The rejecting-window probe reports `clipboard_fallback`, and its text persists after exit. The overlay no longer deadlocks. A cold focused-element query took 78 ms, exceeding the prior 20 ms message limit; this specific preflight now allows a 150 ms message within a 200 ms deadline. Other context attribute reads keep their existing short timeout.
- Repeating TextEdit after adding the real overlay and the bounded preflight verifies the normal insertion path independently. The final build is being regenerated with these fixes; no superseded bundle will be distributed as the final release.
- The full Rust suite passes all 296 tests after the paste and overlay fixes. Native TextEdit verification still inserts exactly, with a 28 ms focused-role query and `clipboard_read`. The negative recipient shows the real clipboard notice and retains exact text. Results are summarized in `verification/native-2026-09-23.json`; reproduction notes are in `verification/NATIVE_CHECKS.md`.

### 0.1.1 release checks completed

- The regenerated production `cargo tauri build` passed with the final paste and overlay fixes. Strict signature checks passed on the build, installed copy, and app extracted from `artifacts/hushtype-0.1.1-apple-silicon.zip` (18,857,175 bytes). Both bundled MIT notices still match their sources. Build log: `/tmp/hushtype-0.1.1-verified-build.log`; release Clippy passed with 53 existing warnings.
- Granted Microphone through its normal dialog and added the exact installed final app through System Settings → Accessibility. Onboarding advanced automatically after the grant. A normal Command-Q and relaunch preserved both grants and the saved model, endpoint, four modes, replacement, and startup preferences. First launch stayed hidden as configured; opening the running app again revealed settings.
- Sent native Option-Space key-down/up events into the installed app's global shortcut handler. A 1.2-second hold completed through `empty_audio` with capture stop 19 ms and no STT, LLM, or paste attempt. A short tap locked recording on; Escape canceled it and returned the tray to Idle. These are keyboard/capture checks with the built-in microphone and no human speech, not completed live dictations.
- Captured the final installed Modes view in `artifacts/hushtype-settings.png`. Light/dark, minimum-width, clipboard-notice, and original-logo images are also available in that directory. The model helper in the extracted app archive launches successfully. The earlier fresh model download independently verified its pinned hash and extraction.
- Verified the downloadable 0.1.1 app archive returns HTTP 200 on devbox1 port 8842. Ollama remains resident on the Tailscale address with qwen3:8b and the 4096-token context. The MacBook has not connected to it during these checks.
- Implementation checkpoints: `b4eeb61` original identity, `38c54ee` permission recovery and settings design, `8757459` native paste fallback, and `e6524ea` persistent optional host and external setup. No new runtime dependency was added.

### Remaining acceptance after Ali deferred live microphone testing

The signed development-host app, setup helper, native settings and permission recovery, browser/app context, TextEdit paste, rejecting-window clipboard notice, endpoint eval, and host restart checks have passed. All 296 Rust tests and both permission regression tests pass; the unchanged Ollama eval passes 27/30. This does not establish universal paste compatibility or production reliability on every Mac.

Still pending on the MacBook: clean macOS 14+ launch, Microphone/Accessibility grants on that machine, connection to devbox1, 20 actual human dictations with end-to-end latency, the live Slack/Mail/Cursor comparison, and a live endpoint-outage dictation that pastes raw without a dialog. Synthetic STT and text-only timings remain diagnostic evidence only. Remote STT and dictionary auto-learning have not been started.
