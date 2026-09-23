# Phase 0 STT smoke check

2026-09-22 on devbox1, macOS 15.7.9, M1 Max. This uses synthetic speech, not a real dictation.

Input text: Send the project update on Wednesday. The build is ready for review.

```sh
say -v Samantha -o /tmp/hushtype-phase0.aiff 'Send the project update on Wednesday. The build is ready for review.'
afconvert -f WAVE -d LEI16@16000 -c 1 /tmp/hushtype-phase0.aiff /tmp/hushtype-phase0.wav
src-tauri/target/release/bundle/macos/hushtype.app/Contents/MacOS/hushtype --transcribe-file /tmp/hushtype-phase0.wav --model parakeet-tdt-0.6b-v3 --repeat 3 --json
```

The exact JSON result is in `phase0-stt.json`. Native build work was running concurrently, so these durations do not establish the latency targets. The desktop was locked. GUI, hotkey, microphone, and paste checks are pending.
