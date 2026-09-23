import { useEffect, useState, useRef } from "react";
import { useTranslation } from "react-i18next";
import { platform } from "@tauri-apps/plugin-os";
import {
  checkAccessibilityPermission,
  requestAccessibilityPermission,
  checkMicrophonePermission,
  requestMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import { toast } from "sonner";
import { relaunch } from "@tauri-apps/plugin-process";
import { Button } from "../ui/Button";
import { commands } from "@/bindings";
import { useSettingsStore } from "@/stores/settingsStore";
import { SetupLayout } from "./SetupLayout";
import { Keyboard, Mic, Check, Loader2 } from "lucide-react";

interface AccessibilityOnboardingProps {
  onComplete: () => void;
  preview?: boolean;
}

type PermissionStatus = "checking" | "needed" | "waiting" | "granted";
type PermissionPlatform = "macos" | "windows" | "other";

interface PermissionsState {
  accessibility: PermissionStatus;
  microphone: PermissionStatus;
}

const AccessibilityOnboarding: React.FC<AccessibilityOnboardingProps> = ({
  onComplete,
  preview = false,
}) => {
  const { t } = useTranslation();
  const refreshAudioDevices = useSettingsStore(
    (state) => state.refreshAudioDevices,
  );
  const refreshOutputDevices = useSettingsStore(
    (state) => state.refreshOutputDevices,
  );
  const [permissionPlatform, setPermissionPlatform] =
    useState<PermissionPlatform | null>(null);
  const [permissions, setPermissions] = useState<PermissionsState>({
    accessibility: "checking",
    microphone: "checking",
  });
  const recheckRef = useRef<(() => Promise<void>) | null>(null);
  const completeRef = useRef(onComplete);
  completeRef.current = onComplete;
  const [showRecovery, setShowRecovery] = useState(false);
  const [checkFailed, setCheckFailed] = useState(false);
  const isMacOS = permissionPlatform === "macos";
  const isWindows = permissionPlatform === "windows";
  const showMicrophonePermission = isMacOS || isWindows;
  const showAccessibilityPermission = isMacOS;
  const allGranted = isMacOS
    ? permissions.accessibility === "granted" &&
      permissions.microphone === "granted"
    : isWindows
      ? permissions.microphone === "granted"
      : true;

  // Recheck while this screen is visible, including grants made directly in
  // System Settings. A permission request's return does not imply a grant.
  useEffect(() => {
    const current = platform();
    const next: PermissionPlatform =
      current === "macos"
        ? "macos"
        : current === "windows"
          ? "windows"
          : "other";
    setPermissionPlatform(next);
    if (preview) {
      setPermissions({
        accessibility: next === "macos" ? "needed" : "granted",
        microphone: next === "other" ? "granted" : "needed",
      });
      return;
    }
    if (next === "other") {
      completeRef.current();
      return;
    }
    let disposed = false;
    let inFlight = false;
    let complete = false;
    let completionTimer: ReturnType<typeof setTimeout> | undefined;
    async function check() {
      if (inFlight || disposed || complete) return;
      inFlight = true;
      try {
        let accessibility = true;
        let microphone = true;
        if (next === "macos") {
          [accessibility, microphone] = await Promise.all([
            checkAccessibilityPermission(),
            checkMicrophonePermission(),
          ]);
        } else {
          const status = await commands.getWindowsMicrophonePermissionStatus();
          microphone = !status.supported || status.overall_access !== "denied";
        }
        if (disposed) return;
        setCheckFailed(false);
        setPermissions((previous) => ({
          accessibility: accessibility
            ? "granted"
            : previous.accessibility === "waiting"
              ? "waiting"
              : "needed",
          microphone: microphone
            ? "granted"
            : previous.microphone === "waiting"
              ? "waiting"
              : "needed",
        }));
        if (accessibility && microphone) {
          await Promise.all([refreshAudioDevices(), refreshOutputDevices()]);
          if (disposed) return;
          complete = true;
          completionTimer = setTimeout(() => completeRef.current(), 300);
        }
      } catch (error) {
        if (!disposed) {
          console.warn("Permission check failed:", error);
          setCheckFailed(true);
          setPermissions((previous) => ({
            accessibility:
              previous.accessibility === "checking"
                ? "needed"
                : previous.accessibility,
            microphone:
              previous.microphone === "checking"
                ? "needed"
                : previous.microphone,
          }));
        }
      } finally {
        inFlight = false;
      }
    }
    recheckRef.current = check;
    void check();
    const interval = setInterval(() => void check(), 1500);
    const onFocus = () => void check();
    window.addEventListener("focus", onFocus);
    const recoveryTimer = setTimeout(() => setShowRecovery(true), 6000);
    return () => {
      disposed = true;
      recheckRef.current = null;
      clearInterval(interval);
      clearTimeout(recoveryTimer);
      if (completionTimer) clearTimeout(completionTimer);
      window.removeEventListener("focus", onFocus);
    };
  }, [preview, refreshAudioDevices, refreshOutputDevices]);

  async function handleGrantAccessibility() {
    if (preview) return;
    try {
      setPermissions((previous) => ({ ...previous, accessibility: "waiting" }));
      await requestAccessibilityPermission();
      await recheckRef.current?.();
    } catch (error) {
      console.error("Failed to request Accessibility:", error);
      toast.error(t("onboarding.permissions.errors.requestFailed"));
    }
  }
  async function handleGrantMicrophone() {
    if (preview) return;
    try {
      setPermissions((previous) => ({ ...previous, microphone: "waiting" }));
      if (isWindows) await commands.openMicrophonePrivacySettings();
      else await requestMicrophonePermission();
      await recheckRef.current?.();
    } catch (error) {
      console.error("Failed to request Microphone:", error);
      toast.error(t("onboarding.permissions.errors.requestFailed"));
    }
  }

  const isChecking =
    permissionPlatform === null ||
    (isMacOS &&
      permissions.accessibility === "checking" &&
      permissions.microphone === "checking") ||
    (isWindows && permissions.microphone === "checking");

  // Still checking platform/initial permissions
  if (isChecking) {
    return (
      <div className="h-screen w-full flex items-center justify-center">
        <Loader2 className="w-8 h-8 animate-spin text-text/50" />
      </div>
    );
  }

  // All permissions granted - show success briefly
  if (allGranted) {
    return (
      <div className="h-screen w-full flex flex-col items-center justify-center gap-4">
        <div className="p-4 rounded-full bg-emerald-500/20">
          <Check className="w-12 h-12 text-emerald-400" />
        </div>
        <p className="text-lg font-medium text-text">
          {t("onboarding.permissions.allGranted")}
        </p>
      </div>
    );
  }

  // Show permissions request screen
  return (
    <SetupLayout
      step={1}
      title={t("onboarding.permissions.title")}
      description={t("onboarding.permissions.description")}
    >
      <div className="w-full">
        {/* Microphone Permission Card */}
        {showMicrophonePermission && (
          <div className="permission-card">
            <div className="flex items-center gap-4">
              <div className="permission-icon shrink-0">
                <Mic className="w-6 h-6 text-logo-primary" />
              </div>
              <div className="flex-1 min-w-0">
                <h3 className="font-medium text-text">
                  {t("onboarding.permissions.microphone.title")}
                </h3>
                <p className="text-sm text-text/60 mb-3">
                  {t("onboarding.permissions.microphone.description")}
                </p>
                {permissions.microphone === "granted" ? (
                  <div className="flex items-center gap-2 text-emerald-400 text-sm">
                    <Check className="w-4 h-4" />
                    {t("onboarding.permissions.granted")}
                  </div>
                ) : permissions.microphone === "waiting" ? (
                  <div className="flex items-center gap-2 text-text/50 text-sm">
                    <Loader2 className="w-4 h-4 animate-spin" />
                    {t("onboarding.permissions.waiting")}
                  </div>
                ) : (
                  <button
                    onClick={handleGrantMicrophone}
                    className="permission-action"
                  >
                    {isWindows
                      ? t("accessibility.openSettings")
                      : t("onboarding.permissions.grant")}
                  </button>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Accessibility Permission Card */}
        {showAccessibilityPermission && (
          <div className="permission-card">
            <div className="flex items-center gap-4">
              <div className="permission-icon shrink-0">
                <Keyboard className="w-6 h-6 text-logo-primary" />
              </div>
              <div className="flex-1 min-w-0">
                <h3 className="font-medium text-text">
                  {t("onboarding.permissions.accessibility.title")}
                </h3>
                <p className="text-sm text-text/60 mb-3">
                  {t("onboarding.permissions.accessibility.description")}
                </p>
                {permissions.accessibility === "granted" ? (
                  <div className="flex items-center gap-2 text-emerald-400 text-sm">
                    <Check className="w-4 h-4" />
                    {t("onboarding.permissions.granted")}
                  </div>
                ) : permissions.accessibility === "waiting" ? (
                  <div className="flex items-center gap-2 text-text/50 text-sm">
                    <Loader2 className="w-4 h-4 animate-spin" />
                    {t("onboarding.permissions.waiting")}
                  </div>
                ) : (
                  <button
                    onClick={handleGrantAccessibility}
                    className="permission-action"
                  >
                    {t("onboarding.permissions.grant")}
                  </button>
                )}
              </div>
            </div>
          </div>
        )}
        {checkFailed && (
          <p role="alert" className="text-sm text-error mt-4">
            {t("onboarding.permissions.errors.checkFailed")}
          </p>
        )}
        {showRecovery && !preview && (
          <div className="permission-recovery">
            <p>
              {t(
                isMacOS && permissions.accessibility !== "granted"
                  ? "hushtype.ui.permissionRecovery"
                  : "hushtype.ui.microphoneRecovery",
              )}
            </p>
            <div className="flex flex-wrap gap-2 mt-3">
              <Button
                variant="secondary"
                onClick={async () => {
                  try {
                    const result =
                      isMacOS && permissions.accessibility !== "granted"
                        ? await commands.openAccessibilityPrivacySettings()
                        : await commands.openMicrophonePrivacySettings();
                    if (result.status === "error")
                      throw new Error(result.error);
                  } catch {
                    toast.error(
                      t("onboarding.permissions.errors.requestFailed"),
                    );
                  }
                }}
              >
                {t("accessibility.openSettings")}
              </Button>
              <Button
                variant="secondary"
                onClick={() => void recheckRef.current?.()}
              >
                {t("hushtype.ui.checkAgain")}
              </Button>
              {isMacOS && (
                <Button
                  variant="secondary"
                  onClick={() =>
                    relaunch().catch(() =>
                      toast.error(t("hushtype.ui.restartFailed")),
                    )
                  }
                >
                  {t("hushtype.ui.restart")}
                </Button>
              )}
            </div>
          </div>
        )}
      </div>
    </SetupLayout>
  );
};

export default AccessibilityOnboarding;
