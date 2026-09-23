import React, { useState, useEffect } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { useTranslation } from "react-i18next";
import { useSettings } from "@/hooks/useSettings";

import ModelSelector from "../model-selector";

const Footer: React.FC = () => {
  const { t } = useTranslation();
  const { settings } = useSettings();
  const [version, setVersion] = useState("");

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

  return (
    <footer className="app-footer">
      <ModelSelector />
      <div className="footer-meta">
        {settings?.refine && (
          <span>
            {t(
              settings.refine.enabled
                ? "hushtype.ui.refineOn"
                : "hushtype.ui.refineOff",
            )}
          </span>
        )}
        {version && <span>{t("hushtype.ui.version", { version })}</span>}
      </div>
    </footer>
  );
};

export default Footer;
