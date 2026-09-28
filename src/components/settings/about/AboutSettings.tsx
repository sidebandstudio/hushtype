import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { toast } from "sonner";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";
import { AppDataDirectory } from "../AppDataDirectory";
import { AppLanguageSelector } from "../AppLanguageSelector";
import { ThemeSelector } from "../ThemeSelector";
import { UpdateChecksToggle } from "../UpdateChecksToggle";
import { LogDirectory } from "../debug";
import { useUpdateStore } from "@/stores/updateStore";

export const AboutSettings: React.FC = () => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");
  const { availableVersion, checking, checkNow, runInstaller } =
    useUpdateStore();

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.error("Failed to get app version:", error);
      }
    };

    fetchVersion();
  }, []);

  const handleCheck = async () => {
    try {
      const newer = await checkNow();
      if (!newer) {
        toast.success(t("settings.about.update.upToDate", { version }));
      }
    } catch (error) {
      console.error("Update check failed:", error);
      toast.error(t("settings.about.update.failed"));
    }
  };

  const handleInstall = async () => {
    try {
      await runInstaller();
    } catch (error) {
      console.error("Failed to start the updater:", error);
      toast.error(t("settings.about.update.installFailed"));
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.about.title")}>
        <AppLanguageSelector descriptionMode="tooltip" grouped={true} />
        <ThemeSelector descriptionMode="tooltip" grouped={true} />
        <SettingContainer
          title={t("settings.about.version.title")}
          description={t("settings.about.version.description")}
          grouped={true}
        >
          {/* eslint-disable-next-line i18next/no-literal-string */}
          <span className="text-sm font-mono">v{version}</span>
        </SettingContainer>
        <SettingContainer
          title={t("settings.about.update.title")}
          description={
            availableVersion
              ? t("settings.about.update.available", {
                  version: availableVersion,
                })
              : t("settings.about.update.description")
          }
          grouped={true}
        >
          {availableVersion ? (
            <Button size="sm" onClick={handleInstall}>
              {t("settings.about.update.install")}
            </Button>
          ) : (
            <Button
              size="sm"
              variant="secondary"
              onClick={handleCheck}
              disabled={checking}
            >
              {t("settings.about.update.check")}
            </Button>
          )}
        </SettingContainer>
        <UpdateChecksToggle descriptionMode="tooltip" grouped={true} />

        <AppDataDirectory descriptionMode="tooltip" grouped={true} />
        <LogDirectory grouped={true} />
      </SettingsGroup>

      <SettingsGroup title={t("settings.about.acknowledgments.title")}>
        <SettingContainer
          title={t("settings.about.acknowledgments.ggml.title")}
          description={t("settings.about.acknowledgments.ggml.description")}
          grouped={true}
          layout="stacked"
        >
          <div className="text-sm text-mid-gray">
            {t("settings.about.acknowledgments.ggml.details")}
          </div>
        </SettingContainer>
      </SettingsGroup>
    </div>
  );
};
