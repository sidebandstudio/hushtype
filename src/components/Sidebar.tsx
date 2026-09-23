import React from "react";
import { useTranslation } from "react-i18next";
import {
  Cog,
  FlaskConical,
  History,
  Info,
  PenLine,
  Cpu,
  BookOpen,
  SlidersHorizontal,
  Mic2,
} from "lucide-react";
import HushtypeWordmark from "./icons/HushtypeWordmark";
import { useSettings } from "../hooks/useSettings";
import type { AppSettings } from "@/bindings";
import {
  GeneralSettings,
  AdvancedSettings,
  HistorySettings,
  DebugSettings,
  AboutSettings,
  RefineSettings,
  DictionarySettings,
  ModesSettings,
  ModelsSettings,
} from "./settings";

export type SidebarSection = keyof typeof SECTIONS_CONFIG;

interface IconProps {
  width?: number | string;
  height?: number | string;
  size?: number | string;
  className?: string;
}

interface SectionConfig {
  labelKey: string;
  icon: React.ComponentType<IconProps>;
  component: React.ComponentType;
  enabled: (settings: AppSettings | null) => boolean;
}

export const SECTIONS_CONFIG = {
  general: {
    labelKey: "hushtype.ui.dictation",
    icon: Mic2,
    component: GeneralSettings,
    enabled: () => true,
  },
  history: {
    labelKey: "sidebar.history",
    icon: History,
    component: HistorySettings,
    enabled: () => true,
  },
  models: {
    labelKey: "sidebar.models",
    icon: Cpu,
    component: ModelsSettings,
    enabled: () => true,
  },
  advanced: {
    labelKey: "sidebar.advanced",
    icon: Cog,
    component: AdvancedSettings,
    enabled: () => true,
  },
  refine: {
    labelKey: "hushtype.refine.title",
    icon: PenLine,
    component: RefineSettings,
    enabled: () => true,
  },
  modes: {
    labelKey: "hushtype.modes.title",
    icon: SlidersHorizontal,
    component: ModesSettings,
    enabled: () => true,
  },
  dictionary: {
    labelKey: "hushtype.dictionary.title",
    icon: BookOpen,
    component: DictionarySettings,
    enabled: () => true,
  },
  debug: {
    labelKey: "sidebar.debug",
    icon: FlaskConical,
    component: DebugSettings,
    enabled: (settings) => settings?.debug_mode ?? false,
  },
  about: {
    labelKey: "sidebar.about",
    icon: Info,
    component: AboutSettings,
    enabled: () => true,
  },
} as const satisfies Record<string, SectionConfig>;

interface SidebarProps {
  activeSection: SidebarSection;
  onSectionChange: (section: SidebarSection) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();

  const order: SidebarSection[] = [
    "general",
    "refine",
    "modes",
    "dictionary",
    "history",
    "models",
    "advanced",
    "debug",
    "about",
  ];
  const availableSections = order
    .filter((id) => SECTIONS_CONFIG[id].enabled(settings))
    .map((id) => ({ id, ...SECTIONS_CONFIG[id] }));

  return (
    <header className="app-header">
      <div className="brand-row">
        <HushtypeWordmark />
        <span className="brand-caption">{t("hushtype.ui.localDictation")}</span>
      </div>
      <nav className="settings-nav" aria-label={t("hushtype.ui.navigation")}>
        {availableSections.map((section) => {
          const Icon = section.icon;
          const isActive = activeSection === section.id;

          return (
            <button
              type="button"
              key={section.id}
              className={`nav-item ${isActive ? "is-active" : ""}`}
              aria-current={isActive ? "page" : undefined}
              aria-controls="settings-content"
              onClick={() => onSectionChange(section.id)}
            >
              <Icon width={16} height={16} aria-hidden="true" />
              <span>{t(section.labelKey)}</span>
            </button>
          );
        })}
      </nav>
    </header>
  );
};
