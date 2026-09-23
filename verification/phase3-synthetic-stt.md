# Synthetic STT diagnostic

This is one generated WAV replayed 20 times, not 20 real dictations. It does not exercise hotkeys, live microphone input, app modes, refinement, or paste.

The final packaged executable ran on devbox1, M1 Max, macOS 15.7.9, after the release build finished. `pmset -g therm` reported no recorded thermal or performance warnings.

- Voice: macOS Samantha, rate 120.
- Format: mono PCM16, 16 kHz, 9.4751875 seconds.
- Engine: Parakeet V3 INT8, ONNX, requested device settings.
- Model load: 772 ms, excluded from the inference timings.
- Twenty repeats: p50 372 ms, p95 412 ms, max 432 ms.
- Exact output: Please send the updated project report on Wednesday, I will review the logs after lunch and follow up with the team before the end of the day.

```sh
src-tauri/target/release/bundle/macos/hushtype.app/Contents/MacOS/hushtype \
  --transcribe-file /tmp/hushtype-phase3-10s.wav \
  --model parakeet-tdt-0.6b-v3 --repeat 20 --json
```

These numbers diagnose engine speed on synthetic audio. The required live latency report remains unmeasured in `phase3-real-latency.json`.
