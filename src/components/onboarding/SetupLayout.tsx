import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import HushtypeWordmark from "../icons/HushtypeWordmark";

export function SetupLayout({
  step,
  title,
  description,
  children,
}: {
  step: 1 | 2;
  title: string;
  description: string;
  children: ReactNode;
}) {
  const { t } = useTranslation();
  return (
    <div className="setup-shell">
      <aside className="setup-aside">
        <HushtypeWordmark />
        <div className="setup-art">
          <span className="brand-symbol" aria-hidden="true" />
          <p>{t("hushtype.ui.holdSpeakRelease")}</p>
        </div>
        <ol className="setup-steps" aria-label={t("hushtype.ui.setup")}>
          {([1, 2] as const).map((number) => (
            <li
              key={number}
              aria-current={step === number ? "step" : undefined}
            >
              <span className="setup-step-number">{number}</span>
              {t(
                number === 1
                  ? "hushtype.ui.permissions"
                  : "hushtype.ui.speechModel",
              )}
            </li>
          ))}
        </ol>
      </aside>
      <main className="setup-main">
        <p className="eyebrow">
          {t("hushtype.ui.setupStep", { number: step })}
        </p>
        <h1>{title}</h1>
        <p className="setup-description">{description}</p>
        {children}
      </main>
    </div>
  );
}
