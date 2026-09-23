// Test-only IPC boundary. This entry point is not included in the app build.
import React from "react";
import { createRoot } from "react-dom/client";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { mockIPC } from "@tauri-apps/api/mocks";
import translations from "../src/i18n/locales/en/translation.json";
import AccessibilityOnboarding from "../src/components/onboarding/AccessibilityOnboarding";
import "../src/App.css";

declare global {
  interface Window {
    permissionTest: {
      accessibility: boolean;
      microphone: boolean;
      requests: number;
      restarts: number;
      completions: number;
    };
  }
}
window.permissionTest = {
  accessibility: false,
  microphone: false,
  requests: 0,
  restarts: 0,
  completions: 0,
};
Object.assign(window, {
  __TAURI_OS_PLUGIN_INTERNALS__: {
    platform: "macos",
    os_type: "macos",
    family: "unix",
  },
});
mockIPC(
  (command) => {
    if (command === "plugin:macos-permissions|check_accessibility_permission")
      return window.permissionTest.accessibility;
    if (command === "plugin:macos-permissions|check_microphone_permission")
      return window.permissionTest.microphone;
    if (command.startsWith("plugin:macos-permissions|request_")) {
      window.permissionTest.requests += 1;
      return;
    }
    if (command === "plugin:process|restart") {
      window.permissionTest.restarts += 1;
      return;
    }
    if (
      command === "get_available_microphones" ||
      command === "get_available_output_devices"
    )
      return [];
    throw new Error(`Unexpected test IPC: ${command}`);
  },
  { shouldMockEvents: true },
);
await i18n.use(initReactI18next).init({
  lng: "en",
  resources: { en: { translation: translations } },
  interpolation: { escapeValue: false },
});
const root = createRoot(document.getElementById("root")!);
root.render(
  <AccessibilityOnboarding
    onComplete={() => {
      window.permissionTest.completions += 1;
      root.render(<p role="status">Permissions passed</p>);
    }}
  />,
);
