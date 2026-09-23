# Refine eval

Run `./eval/run.sh` with Ollama running and `qwen3:8b` installed. The runner reads the app's endpoint settings and editable `refine.txt`, falling back to the bundled defaults when those files do not exist. It compiles the production Rust refine module directly.

To use another configured endpoint, set `HUSHTYPE_REFINE_BASE_URL`, `HUSHTYPE_REFINE_MODEL`, and optionally `HUSHTYPE_REFINE_API_KEY`, `HUSHTYPE_REFINE_TIMEOUT_MS`, or `HUSHTYPE_REFINE_PROMPT`. Never put a real key in committed scripts or results.

Each fixture has a raw transcript and a fixed expected output. The score requires a successful refinement and exact wording/punctuation, ignoring whitespace and curly versus straight apostrophes. Raw fallback never earns a pass. The runner prints differences, writes `eval/latest.json`, and exits unsuccessfully below 27/30.

One warmup request allows up to 120 seconds to load the model and is excluded from scores and timings. The 30 fixtures use the configured timeout, 2500 ms by default. Use `--no-warmup` to exercise cold or unavailable endpoints.

The six `Transcript:` / `Cleaned:` pairs in the plain prompt file become user/assistant conversation examples. Edit the instructions or pairs without rebuilding the app. These examples use different sentences from the eval fixtures.

`cargo test --manifest-path eval/Cargo.toml` exercises real local sockets for request construction, timeout, server failure, parameter negotiation, and output rejection. Socket test servers are confined to test code.

The committed Phase 1 result is 27/30 on local Ollama. One date correction falls back to raw because the model chose the abandoned date. The other two misses are punctuation/capitalization differences. These text-only results are not microphone, paste, or end-to-end latency measurements.

`cargo run --release --manifest-path eval/Cargo.toml --bin modes` compares the same sentence through the shipped Messaging, Email, and Code modes on local Ollama. The app identities in this comparison are explicit fixtures. It does not exercise native context capture or paste into those apps.

The final Phase 3 regression result also passes 27/30. Long-input tests cover smaller-chunk retries, one shared deadline, and discarding partial edits if a later chunk fails.
