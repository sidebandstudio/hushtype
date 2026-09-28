// Test-only IPC boundary. This entry point is not included in the app build.
import { createRoot } from "react-dom/client";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { mockIPC } from "@tauri-apps/api/mocks";
import translations from "../src/i18n/locales/en/translation.json";
import { ModesSettings } from "../src/components/settings/modes/ModesSettings";
import type { Mode } from "../src/bindings";
import "../src/App.css";

declare global {
  interface Window {
    savedModes: Mode[] | null;
  }
}
const context = {
  app: true,
  window_title: true,
  url: true,
  selected_text: false,
  clipboard: false,
};
const mode = (id: string, name: string, bundles: string[]): Mode => ({
  id,
  name,
  bundle_ids: bundles,
  url_globs: [],
  refine_enabled: id !== "code",
  prompt: "",
  prompt_behavior: "append",
  context,
  style: id === "messaging" ? "casual" : "serious",
});
let modes = [
  mode("messaging", "Messaging", ["com.apple.MobileSMS", "com.hnc.Discord"]),
  mode("email", "Email", ["com.apple.mail"]),
  mode("default", "Default", []),
];
window.savedModes = null;
mockIPC(
  (command, args) => {
    if (command === "get_app_settings" || command === "get_default_settings")
      return { modes };
    if (command === "check_custom_sounds") return { start: false, stop: false };
    if (command === "is_update_checks_locked") return true;
    if (command === "update_modes") {
      modes = (args as { modes: Mode[] }).modes;
      window.savedModes = modes;
      return null;
    }
    throw new Error(`Unexpected test IPC: ${command}`);
  },
  { shouldMockEvents: true },
);
await i18n.use(initReactI18next).init({
  lng: "en",
  resources: { en: { translation: translations } },
  interpolation: { escapeValue: false },
});
createRoot(document.getElementById("root")!).render(<ModesSettings />);
