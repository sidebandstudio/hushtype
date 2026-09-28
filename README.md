# hushtype

Hold a shortcut, speak, and release to paste cleaned text at the cursor. hushtype runs speech recognition locally and optionally sends the transcript to an OpenAI-compatible endpoint for cleanup. It supports Apple Silicon Macs running macOS 14 or later.

hushtype is proprietary software by Sideband Studio; see [LICENSE](LICENSE). Third-party components and the notices their licenses require are listed in [THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES).

The app has no accounts, sync, or telemetry. Its runtime network requests go to the LLM endpoint you configure and, unless you turn update checks off in About, to hushtype.vercel.app for a small file naming the current version. Model installation happens separately through the setup commands below. Transcription history stays on this Mac; its retention controls are in Advanced settings.

## Setup with Ollama

Configuration takes about five minutes after the initial model downloads and app build.

1. Install [Ollama](https://ollama.com/) and start it locally. For a command-line installation:

   ```sh
   OLLAMA_NO_CLOUD=1 ollama serve
   ```

   In another terminal:

   ```sh
   ollama pull qwen3:8b
   ```

2. Install hushtype. On a Mac with Apple silicon, open Terminal and run:

   ```sh
   curl -fsSL https://hushtype.vercel.app/install.sh | bash
   ```

   The installer downloads the app from [hushtype.vercel.app](https://hushtype.vercel.app) and Parakeet V3 from Hugging Face, checks every file's SHA-256, installs hushtype into Applications, and opens it. Running it again updates the app. From a source checkout, `scripts/setup-models.sh` installs the speech model and the build-time VAD resource instead. The app does not download models. Select Parakeet V3 during onboarding, using Refresh models if needed.

3. Open `hushtype.app` and grant Microphone and Accessibility access in System Settings. These grants must be made on the Mac running hushtype. Install the app in its permanent location before granting access. Keep the desktop unlocked for dictation.

4. Open Refine settings. The defaults are base URL `http://localhost:11434/v1`, model `qwen3:8b`, no API key, and a 2500 ms timeout. Save any changes. The cleanup prompt is editable in settings. Edits are saved to `refine.txt` in the app data folder; without that file, hushtype uses its built-in prompt, which improves with app updates.

5. Place the cursor in a text field. Hold the configured shortcut, speak, then release. The default is Option-Space; check General settings for the active binding. A short tap can toggle recording with the default hold-or-toggle behavior. Choose hold-only there if preferred.

The timeout covers a short dictation; longer dictations get proportionally more time, up to four times the setting. A cold LLM can take longer than that to load, so the first dictation may paste raw text. Keep Ollama and the model warm for latency measurements. Endpoint errors, timeouts, and rejected edits fall back to the complete raw transcript without an LLM error dialog. Explicit dictionary replacements still apply.

## MacBook with a separate local host

Run hushtype and Parakeet on the MacBook where you speak and type. The shortcut, microphone, speech recognition, and paste all run there. devbox1 can run Ollama for optional cleanup. Audio stays on the MacBook; only the transcript and enabled mode context travel to that host over Tailscale.

On the host, install Ollama and download `qwen3:8b` once, then run:

```sh
scripts/setup-refine-host.sh
```

This installs the user launch agent `dev.hushtype.ollama`. It binds to the host's Tailscale IPv4 address, disables Ollama cloud features, preloads the model, keeps it loaded, and restarts the service when it exits. It uses a 4096-token context. These settings follow the [Ollama configuration documentation](https://docs.ollama.com/faq). The service prevents automatic system sleep on AC power while it runs.

On the MacBook, connect Tailscale and set Refine to:

| Setting  | devbox1 value                  |
| -------- | ------------------------------ |
| Base URL | `http://100.80.149.7:11434/v1` |
| Model    | `qwen3:8b`                     |
| API key  | Leave empty                    |
| Timeout  | `2500` ms                      |

Enable **Launch on Startup** and **Start Hidden** in Advanced on the MacBook. The host service starts after its user signs in. Keep the host powered and signed in; it cannot serve while shut down, explicitly asleep, or waiting at FileVault login. If the host or network is unavailable, hushtype still transcribes and pastes locally with raw text. You can also run Ollama on the MacBook using the default localhost URL.

To inspect or restart the host:

```sh
launchctl print "gui/$(id -u)/dev.hushtype.ollama"
launchctl kickstart -k "gui/$(id -u)/dev.hushtype.ollama"
```

Host logs are in `~/Library/Logs/dev.hushtype.host/server.log`. To remove the service, run `launchctl bootout "gui/$(id -u)/dev.hushtype.ollama"` and delete `~/Library/LaunchAgents/dev.hushtype.ollama.plist`.

## Permission recovery

Onboarding checks the real permission state continuously and when you return from System Settings. If a permission is already enabled but the app still waits, use **Restart hushtype**. For Accessibility, remove the old hushtype entry in System Settings and add the installed copy again.

An ad-hoc rebuild can invalidate an earlier grant. If re-adding the app still leaves a stale entry, quit hushtype, run `tccutil reset Accessibility dev.hushtype.app`, then reopen it and grant Accessibility again. This resets only hushtype's Accessibility record. Microphone access and other apps' permissions are unaffected.

## Other endpoints

In Refine, set the base URL, optional API key, and model name accepted by your server. For a local proxy use `http://127.0.0.1:10531/v1`. For OpenAI use `https://api.openai.com/v1` with your API key and an available model. Do not include `/chat/completions` in the base URL; hushtype appends it. There are no provider switches.

The client requests nonstreaming chat completions and negotiates unsupported optional parameters from server errors. Redirects and environment proxy discovery are disabled. Keys use the existing local settings store and are omitted from debug formatting. Choose your endpoint with the understanding that it receives the transcript and the context enabled below.

Long dictations split at sentence or word boundaries. Maximum characters per request defaults to 4000; reduce it for a small context window. Explicit context-limit errors trigger smaller chunks. All requests share one deadline. If the full cleanup cannot finish, the entire raw transcript is kept.

## Modes and dictionary

Modes match in saved order by bundle ID or URL glob. `*` matches any sequence and `?` matches one character. The first match wins; Default is always last. Each mode controls refinement, prompt additions or replacement, and which context reaches the endpoint.

- Messaging covers Slack, Messages, Discord, Telegram, and WhatsApp, with casual wording and no added sign-off.
- Email covers Mail, Gmail, Outlook, and Superhuman, with complete sentences and paragraphs.
- Code covers Cursor, VS Code, Xcode, iTerm, Terminal, Warp, and Ghostty. Refinement starts off; its optional prompt preserves literal tokens and identifiers.
- Default performs standard cleanup. Turning Refine off globally disables it in every mode.

Each mode also has a style. Serious keeps capitals and full punctuation. Casual drops the closing period. Very casual also lowercases sentence starts and "i", but keeps names capitalized. Styles never change words, and they apply even when refinement is off or falls back to raw text. Messaging starts as Casual; the other modes start as Serious.

App identity is captured at recording start. Native Accessibility supplies window titles and browser URLs for Safari, Chrome, Arc, and Brave where exposed. Missing or denied context stays absent. App identity, title, and URL are enabled as context by default. Selected text and clipboard text start off and are read for context only when the matched mode enables them. Context is not written into history or timing logs. History retry uses Default without recapturing the settings window.

Dictionary terms feed supported Whisper prompts, local spelling correction, and refinement. Local correction runs before refinement and also joins split words, so add names that speech recognition mishears, such as `hushtype` or `Wispr Flow`. Replacements run afterwards in listed order, matching literal phrases without case sensitivity and respecting word boundaries. For example, `open claw` can become `OpenClaw`.

On macOS, clipboard paste first checks the focused control’s Accessibility role without reading its text. Known non-text controls keep the transcript on the clipboard and show a Command-V notice. Other controls use the existing native read-receipt mechanism. If no reader responds before its timeout, the transcript stays on the clipboard and the same notice appears. A newer clipboard copy is left untouched. A receipt proves that a consumer read the text; it cannot prove that a particular app inserted it.

## Build and checks

Install Rust, Bun, CMake, and Xcode or its command-line tools. With full Xcode installed, this host uses:

```sh
bun install --frozen-lockfile
cargo install tauri-cli --version 2.11.0 --locked
scripts/setup-models.sh
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo tauri build
```

The ad-hoc signed bundle is `src-tauri/target/release/bundle/macos/hushtype.app`. `LICENSE` and `THIRD_PARTY_NOTICES` are inside its Resources folder. The build is not notarized.

```sh
./eval/run.sh
cargo test --manifest-path src-tauri/Cargo.toml --lib
bun run lint
scripts/report-latency.py
```

[Eval instructions and recorded results](eval/README.md) explain the 30 fixed fixtures and endpoint overrides. Refine settings show the last 20 timing records. The rolling log is `~/Library/Logs/dev.hushtype.app/dictation-timings.jsonl`; it includes stage durations, mode, and outcome codes, without transcript or captured context. The latency report needs 20 actual completed dictations and includes raw fallbacks.

## Website and releases

The public site in `website/` is static and deploys to the Sideband Studio Vercel team as the `hushtype` project. To publish a release, build the app as above, then:

```sh
scripts/generate-third-party-notices.sh   # when dependencies changed
scripts/package-website-release.sh        # zips the app, stamps install.sh
cd website && vercel deploy --prod --scope sidebandstudio
```

See [website/README.md](website/README.md) for details.

The architecture, decisions, and verification record are in [ARCHITECTURE.md](ARCHITECTURE.md), [DECISIONS.md](DECISIONS.md), and [PROGRESS.md](PROGRESS.md). Live microphone, three-app, and clean-machine checks remain pending until performed; text-only evals and synthetic audio do not count toward them.
