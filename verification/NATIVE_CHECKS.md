# Native checks

These checks use the real macOS Accessibility and paste APIs. They do not count as human dictations or a clean-machine test. Run them on an unlocked desktop with Accessibility granted to the command runner.

The Cargo example includes the production modules without adding a test command to the shipped app. It checks that the requested bundle is still frontmost before sending anything. It never sends Return. Use a temporary document, not a message composer.

```sh
# Start the actual overlay frontend for the debug example.
bun run dev

# In another terminal, from src-tauri:
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo build --example native_paste_check
```

Open a temporary TextEdit document, focus its text field, and run:

```sh
src-tauri/target/debug/examples/native_paste_check \
  com.apple.TextEdit 'hushtype native paste verification.'
```

Check the document itself for the exact sentence; `clipboard_read` alone is insufficient evidence of insertion. The developer example leaves the real fallback overlay alive for inspection before exiting.

`native-paste-target.swift` is the negative test recipient. Compile it into a temporary `.app` with bundle identifier `dev.hushtype.paste-target`, launch it, and run the same example with that identifier. Its focused window ignores keyboard input. The result should be `clipboard_fallback`, the notice should appear, and the full test text should remain on the clipboard after the example exits.

For browser context, open `https://example.com/?hushtype=context-check` in the browser under test, activate it, and run the native context test with explicit expectations:

```sh
HUSHTYPE_CONTEXT_BUNDLE=com.apple.Safari \
HUSHTYPE_CONTEXT_URL='https://example.com/?hushtype=context-check' \
  cargo test --manifest-path src-tauri/Cargo.toml --lib \
  context::platform::tests::native_capture -- --nocapture
```

Build first if necessary, then reactivate the target before executing the resulting test binary. Compilation can take long enough for focus to change. Chrome uses `com.google.Chrome`; Brave uses `com.brave.Browser`. `HUSHTYPE_CONTEXT_MODE` also checks the matched mode. The probe never reads selected text or clipboard context.
