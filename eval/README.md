# Refine eval

Run `./eval/run.sh` with Ollama running and `qwen3:8b` installed. The runner reads the app's endpoint settings and editable `refine.txt`, falling back to the bundled defaults when those files do not exist. It compiles the production Rust refine module directly.

To use another configured endpoint, set `HUSHTYPE_REFINE_BASE_URL`, `HUSHTYPE_REFINE_MODEL`, and optionally `HUSHTYPE_REFINE_API_KEY`, `HUSHTYPE_REFINE_TIMEOUT_MS`, or `HUSHTYPE_REFINE_PROMPT`. Never put a real key in committed scripts or results.

There are two suites. `fixtures/cleanup.json` has 30 short cleanup cases. `fixtures/disfluency.json` has 18 restart, stutter, and preservation cases, including real Parakeet transcripts from live dictations. A fixture either gives an exact `expected` output or, for long real dictations, phrases that must be `contains`ed or `absent`. It can also name a shipped `mode` and send app `context` the way the app does. The score requires a successful refinement and matching text, ignoring whitespace and curly versus straight apostrophes. Raw fallback never earns a pass. The runner prints differences, writes `eval/latest.json`, and exits unsuccessfully when any suite scores below 90% (27/30 was the original bar). `--suite disfluency` runs one suite.

One warmup request allows up to 120 seconds to load the model and is excluded from scores and timings. The fixtures use the app's deadline: the configured timeout (2500 ms by default), scaled up for inputs longer than 200 characters. Use `--no-warmup` to exercise cold or unavailable endpoints.

The `Transcript:` / `Cleaned:` pairs in the plain prompt file become user/assistant conversation examples. Edit the instructions or pairs without rebuilding the app. These examples use different sentences from the eval fixtures. Like the app, the runner ignores an app-data `refine.txt` that is an unedited copy of an earlier bundled prompt.

`cargo test --manifest-path eval/Cargo.toml` exercises real local sockets for request construction, timeout, server failure, parameter negotiation, and output rejection. Socket test servers are confined to test code.

The committed Phase 1 result is 27/30 on local Ollama. One date correction falls back to raw because the model chose the abandoned date. The other two misses are punctuation/capitalization differences. These text-only results are not microphone, paste, or end-to-end latency measurements.

`cargo run --release --manifest-path eval/Cargo.toml --bin modes` compares the same sentence through the shipped Messaging, Email, and Code modes on local Ollama. The app identities in this comparison are explicit fixtures. It does not exercise native context capture or paste into those apps.

The final Phase 3 regression result also passes 27/30. Long-input tests cover smaller-chunk retries, one shared deadline, and discarding partial edits if a later chunk fails.

The Phase 4 prompt revision targets restarts and broken-off words, such as the live dictation "so you do act you actually do not need internet". On local qwen3:8b with identical fixtures and request layout, the earlier prompt scores 27/30 and 12/18 (`results/phase4-before-old-prompt-qwen3-8b.json`); the revised prompt scores 27/30 and 17/18 (`results/phase4-after-qwen3-8b.json`), with the same p50 of about 350 ms. The remaining disfluency miss collapses the grammatical "that that". Dictionary spellings are not an LLM responsibility: the transcription stage's fuzzy matcher already corrects them before refine.

To compare another model, install it on the endpoint and run `HUSHTYPE_REFINE_MODEL=<name> ./eval/run.sh`.
