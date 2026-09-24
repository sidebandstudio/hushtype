# Decisions

## 2026-09-21, before application changes

- Start from the base engine source attributed in `THIRD_PARTY_NOTICES`. Superseded on 2026-09-23: hushtype no longer tracks, merges from, or keeps the history of that source.
- Distribute the base engine's license notice with the app. Superseded on 2026-09-23: the notice moved to `THIRD_PARTY_NOTICES`, hushtype's own `LICENSE` became proprietary, and the About credit was removed.
- Keep the current audio, hotkey, STT, and paste implementations. Insert refinement at the shared post-STT output function. Use `custom_words` as the dictionary instead of introducing a competing list.
- Use the existing React settings components and Tauri store. Keep the app identity lowercase: `hushtype`, bundle ID `dev.hushtype.app`.
- Target macOS 14+ on Apple Silicon. Do not add Windows/Linux work beyond keeping existing conditional code coherent.
- The app's endpoint-only rule includes setup: disable update checks and in-app remote model downloads/discovery. Install STT assets with a documented setup script outside the running app, then use the existing local model loading. No analytics or implicit provider discovery.
- Build tooling needed on this host: Bun runs the existing locked frontend toolchain; CMake compiles the existing native STT dependency; Tauri CLI drives real macOS app builds. These are development tools, not new runtime dependencies.
- Ollama is a local evaluation/setup tool required by the brief, not an application dependency. Use its OpenAI-compatible endpoint through the same generic client as every other server.
- Live microphone dictation, app launch, file-based STT, and text-only eval are separate checks. Never report generated speech or WAV replay as one of the required 20 real dictations. A clean-machine launch requires an actual second clean host or a suitable VM.
- Commit authorship belongs to Ali alone. No issue, discussion, PR, or message is sent to the base engine's project.

## 2026-09-22, identity and local setup

- Start hushtype at version 0.1.0. Internal library names initially stayed unchanged where they did not appear in UI; superseded on 2026-09-23, when they were renamed too.
- Replace the vector wordmark with text in the existing font. Keep the inherited icon until a separate identity change is requested (replaced by the original h mark on 2026-09-23).
- Hide noninstalled models from the frontend and reject the old download command at the IPC boundary. The downloader internals initially remained, unreachable through that command; they were removed on 2026-09-23. The setup script downloads and verifies Parakeet V3 outside the app.

- The locked desktop prevents Phase 0 GUI verification. Record that gate as pending while continuing independently testable work; do not infer that a process launch proves a functioning webview, hotkey, or paste.

## 2026-09-22, Phase 1

- The default shortcut uses refinement when enabled. Keep one endpoint configuration with an optional key, one model, and a whole-operation timeout. Reuse Reqwest, Tokio, Serde, Regex, and Specta already in the app; no new runtime dependencies.
- Request `reasoning_effort: none` uniformly. If a server rejects that parameter, retry without it; negotiate `max_tokens` to `max_completion_tokens` only after an explicit parameter error. All retries share the original deadline. There are no provider-name or hostname branches. Sources checked: [OpenAI chat-completions reference](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create) and [Ollama compatibility reference](https://docs.ollama.com/api/openai-compatibility).
- Disable HTTP redirects and environment proxy discovery in the refine client so a configured endpoint cannot silently send transcripts to another host.
- Keep the original prompt in `src-tauri/resources/refine.txt`. Copy it once to the app data directory as an editable plain file; read that file on each refinement. Missing or invalid prompt/configuration means raw fallback.
- Hygiene is conservative: reject large expansion, novel vocabulary, changed numeric tokens, missing negations, apparent answers, incomplete completions, and empty text. This is a measurable guard, not proof of semantic equivalence.
- Apply explicit dictionary replacements after both successful refinement and raw fallback. With no configured replacements, fallback preserves the STT text exactly.

- The eval crate compiles the same production refine module without linking the desktop/audio stack. It reuses existing app dependencies and adds no runtime dependency.
- Send dictation as a JSON string field to distinguish it from instructions. Request deterministic temperature zero; remove it only if the endpoint explicitly rejects that parameter.

## 2026-09-22, Phase 2

- Match modes in saved order. Bundle IDs and URL globs are alternatives; the first match wins. The Default mode is always last and cannot be removed. Glob syntax is limited to `*` and `?`, using the existing Regex dependency.
- Capture frontmost app identity at recording start. Read native Accessibility context on a bounded background worker while recording, so slow apps do not delay audio capture. A missing attribute stays absent; it does not prevent dictation.
- Capture browser URL through native Accessibility window/document/web-area or identified address-bar attributes. Do not run AppleScript. Default context sends app identity, window title, and URL. Selected text and clipboard require explicit per-mode opt-in before either is read for context.
- Clip context fields and avoid logging or storing them with history. History retry uses Default mode without recapturing selected text or clipboard from the settings window.
- Global Refine off overrides every mode. Code mode additionally defaults to off. Mode prompt additions precede the few-shot examples; a replacement prompt still goes through the same output guard.
- Native APIs reuse existing Objective-C bindings plus direct ApplicationServices/CoreFoundation declarations. No new dependency.

## 2026-09-22, Phase 3

- Add a conservative near-silence check after capture stops, leaving the recorder/VAD intact. It rejects recordings under 100 ms or with no 20 ms frame above -66 dBFS RMS.
- Split long transcripts at sentence/word boundaries, with a configurable maximum of 4000 characters per request by default. Retry smaller chunks after an explicit context-limit error. Every chunk and retry shares the original timeout; any failure returns the entire raw transcript, never a partial result.
- Use the existing native receipt-based clipboard paste on macOS. If no read arrives, leave plain text on the clipboard and show a short nonactivating overlay. Never overwrite a newer user clipboard copy. This extends the existing paste transaction; it does not replace its injection mechanism.
- End timing at paste settlement. A clipboard read is evidence that a consumer requested the text, not proof that the target inserted it; record that distinction. Keep the existing quiet period and timeout rather than tuning them without live-app evidence.
- The menu bar indicator shows only Idle, Listening, Transcribing, or Refining. Existing menu actions remain available.

## 2026-09-23, requested visual identity and desktop testing

- Ali explicitly requested a new logo and a UI that looks different from the base engine's app. This replaces the earlier instruction to keep the original visual design; the existing components, settings store, and dictation engine remain in use.
- Use a warm ivory and ink-green palette, Avenir Next body text and Baskerville headings from macOS, horizontal settings navigation, and an original rounded h mark. No font downloads, component library, or runtime dependency is added.
- Generate the logo and Dock icon with the built-in image tool, save the production assets in this repository, and derive packaging sizes with the existing Tauri icon tooling. Keep prompt provenance with the assets.
- Desktop unlock is verified. Treat permission grants, usable rendered windows, synthetic audio through the microphone, and live human dictations as separate evidence.
- Run hushtype on the Mac where the user speaks and types. That Mac handles the microphone, hotkey, local speech recognition, and paste; devbox1 may host only the optional Ollama refinement endpoint over Tailscale. A server outage therefore preserves local raw dictation. Remote STT remains deferred.
- Use a user launch agent to restart the devbox1 Ollama service, with cloud disabled and the model kept warm. Bind to the machine's Tailscale address so the inference endpoint is not exposed on its ordinary LAN interfaces. No runtime app dependency is added.
- Set the optional host’s Ollama context to 4096 tokens and launchd process type to Interactive: the initial background-priority/default-context service timed out, while the tuned service passes the unchanged 30-fixture gate. This configures the external Ollama host; the app’s generic endpoint client remains provider-independent.
- Ad-hoc builds can leave stale macOS permission grants tied to an earlier binary. The app must keep checking the real OS result and offer normal recovery, never skip onboarding based on a stored boolean. Document an app-specific `tccutil` reset as a last recovery step; do not reset unrelated permissions or modify the TCC database.
- Native testing confirmed that an unrelated clipboard consumer can acknowledge a read even when the target ignores Paste. Before publishing a paste transaction, check only the focused Accessibility role. Recognized non-text controls go straight to clipboard fallback and a notice. Unknown/custom controls retain the existing transaction. This does not read target text, selected text, or clipboard context, and it cannot prove insertion into every custom editor.

## 2026-09-23, Sideband Studio repository

- Create `sidebandstudio/hushtype` as a private repository because Ali requested organization ownership without specifying public visibility.
- Use `origin` for the Sideband Studio repository and keep `hushtype` as the project’s default branch. License and history handling are superseded by the standalone decision below.

## 2026-09-23, standalone product

- Ali's standing instruction: hushtype is a standalone product that carries nothing of the project it started from, beyond what that project's license requires.
- Replace the repository history with a single root commit authored by Ali. Remove the old remote, its branches, and its tags. A bundle of the previous history is kept outside the repository at `~/backups/hushtype-pre-standalone-2026-09-23.bundle` on devbox1.
- The base engine's MIT notice is required wherever its code ships. Keep it, with the Silero VAD and Parakeet notices, in `THIRD_PARTY_NOTICES`, bundled into the app. Nothing in the UI or README names it. hushtype's own `LICENSE` is proprietary, all rights reserved, held by Ali Younes (Sideband Studio).
- `scripts/setup-models.sh` downloads each model file from its original publisher at a pinned revision and checks its SHA-256: Parakeet V3 from the `istupakov/parakeet-tdt-0.6b-v3-onnx` Hugging Face conversion of NVIDIA's model, Silero VAD v4 from the `snakers4/silero-vad` v4.0 release. These files are byte-identical to the earlier install. The app contains no downloader, model catalog, or remote model URL.
- Replace the forked VAD crate with an in-tree Silero wrapper on the ONNX Runtime already linked for transcription; its probabilities are bit-identical to the previous engine on recorded speech, silence, noise, and hum. Use `rodio` and `tao` from crates.io instead of forks; macOS behavior is unchanged. The native shortcut, ONNX transcription, and transcribe-cpp engines stay ordinary crates.io dependencies.
- Rename internal identifiers: `hushtype_lib`, `HUSHTYPE_*` environment variables, and the `native` keyboard implementation. A stored value from before the rename is salvaged to the default, which is the same native implementation.
- Replace inherited feedback sounds with sounds generated by `scripts/generate-feedback-sounds.py`, remove inherited icons and accent colors, and delete the inherited CI workflows, Nix packaging, contributor documents, and release notes. The repository has no CI until one is set up for hushtype.

## 2026-09-23, public website and installer

- Publish hushtype at hushtype.vercel.app, a static site in `website/` deployed to the Sideband Studio Vercel team. It has no analytics, no cookies, and no third-party requests: fonts are self-hosted and a strict Content-Security-Policy is set. `hushtype.sideband.studio` needs a CNAME at the DNS provider for sideband.studio, which is outside Vercel.
- Only an Apple Development certificate is available, so the app stays ad-hoc signed and cannot be notarized. macOS blocks a browser-downloaded ad-hoc app, so the site installs with one Terminal command (`curl ... | bash`); curl downloads are not quarantined. A Developer ID certificate plus notarization would allow an ordinary DMG download later.
- The installer checks for Apple silicon, macOS 14+, and disk space; verifies the app zip and all five Parakeet files by SHA-256; installs into `/Applications`, falling back to `~/Applications`; and opens the app. On an update it closes the running app and resets only hushtype's Accessibility entry, because macOS ties that grant to the exact build.
- Detect the visitor's platform in the browser. Windows visitors get a pop-up saying Windows support is coming soon; iPhone and iPad visitors see that the iPhone version is on the way. Everyone can still open the Mac instructions.
- The public release starts at 0.2.x. Each release gets a new zip name, because zips are served as immutable.
- Distributing the app publicly requires complete notices. `scripts/generate-third-party-notices.sh` regenerates the dependency part of `THIRD_PARTY_NOTICES` from cargo-about, the frontend's production dependency tree, and the statically linked ONNX Runtime and ggml licenses.
