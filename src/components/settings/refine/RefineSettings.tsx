import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type RefineSettings as RefineConfig } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import { SettingsGroup } from "@/components/ui/SettingsGroup";
import { Input } from "@/components/ui/Input";
import { Textarea } from "@/components/ui/Textarea";
import { ToggleSwitch } from "@/components/ui/ToggleSwitch";
import { Button } from "@/components/ui/Button";
import { DictationTimings } from "../debug/DictationTimings";
import { ChevronDown } from "lucide-react";

export function RefineSettings() {
  const { t } = useTranslation();
  const { settings, refreshSettings } = useSettings();
  const [draft, setDraft] = useState<RefineConfig | null>(null);
  const [prompt, setPrompt] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (settings?.refine) setDraft(settings.refine);
  }, [settings?.refine]);
  useEffect(() => {
    let active = true;
    commands
      .getRefinePrompt()
      .then((result) => {
        if (!active) return;
        if (result.status === "ok") setPrompt(result.data);
        else setError(result.error);
      })
      .catch((e: unknown) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, []);

  async function save(e: React.FormEvent) {
    e.preventDefault();
    if (!draft || prompt === null) return;
    setBusy(true);
    setError("");
    setSaved(false);
    try {
      const promptResult = await commands.setRefinePrompt(prompt);
      if (promptResult.status === "error") throw new Error(promptResult.error);
      const result = await commands.updateRefineSettings(draft);
      if (result.status === "error") throw new Error(result.error);
      await refreshSettings();
      setSaved(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  if (!draft) return <p role="status">{t("common.loading")}</p>;
  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <form
        onSubmit={save}
        onChange={() => setSaved(false)}
        className="space-y-6"
      >
        <SettingsGroup
          title={t("hushtype.refine.title")}
          description={t("hushtype.refine.description")}
        >
          <ToggleSwitch
            label={t("hushtype.refine.enabled")}
            description={t("hushtype.refine.fallback")}
            descriptionMode="inline"
            grouped
            checked={draft.enabled}
            onChange={(enabled) => setDraft({ ...draft, enabled })}
            disabled={busy}
          />
          <div className="endpoint-fields">
            {(["base_url", "model", "api_key"] as const).map((field) => (
              <label
                key={field}
                className={`grid gap-1 text-sm ${field === "base_url" ? "field-wide" : ""}`}
              >
                {t(`hushtype.refine.${field}`)}
                <Input
                  type={
                    field === "api_key"
                      ? "password"
                      : field === "base_url"
                        ? "url"
                        : "text"
                  }
                  autoComplete="off"
                  value={draft[field]}
                  required={field !== "api_key"}
                  disabled={busy}
                  onChange={(e) =>
                    setDraft({ ...draft, [field]: e.target.value })
                  }
                />
              </label>
            ))}
            <label className="grid gap-1 text-sm">
              {t("hushtype.refine.timeout")}
              <Input
                type="number"
                min={100}
                max={120000}
                step={100}
                value={draft.timeout_ms}
                disabled={busy}
                onChange={(e) =>
                  setDraft({ ...draft, timeout_ms: Number(e.target.value) })
                }
                required
              />
              <span className="text-xs text-mid-gray">
                {t("hushtype.refine.timeoutDescription")}
              </span>
            </label>
            <label className="grid gap-1 text-sm">
              {t("hushtype.refine.maxInputChars")}
              <Input
                type="number"
                min={256}
                max={32768}
                value={draft.max_input_chars}
                disabled={busy}
                onChange={(e) =>
                  setDraft({
                    ...draft,
                    max_input_chars: Number(e.target.value),
                  })
                }
                required
              />
              <span className="text-xs text-mid-gray">
                {t("hushtype.refine.chunkDescription")}
              </span>
            </label>
          </div>
        </SettingsGroup>
        <SettingsGroup>
          <details>
            <summary className="disclosure-heading">
              {t("hushtype.refine.prompt")}
              <ChevronDown size={16} aria-hidden="true" />
            </summary>
            <div className="p-5 pt-0 space-y-3">
              <p className="text-sm text-mid-gray">
                {t("hushtype.refine.promptDescription")}
              </p>
              <Textarea
                aria-label={t("hushtype.refine.prompt")}
                rows={12}
                className="w-full font-mono text-xs"
                value={prompt ?? ""}
                onChange={(e) => setPrompt(e.target.value)}
                disabled={busy || prompt === null}
                required
              />
              <Button
                type="button"
                variant="secondary"
                onClick={() => {
                  commands
                    .openAppDataDir()
                    .then((r) => {
                      if (r.status === "error") setError(r.error);
                    })
                    .catch((e: unknown) => setError(String(e)));
                }}
              >
                {t("hushtype.refine.openFolder")}
              </Button>
            </div>
          </details>
        </SettingsGroup>
        {error && (
          <p role="alert" className="text-sm text-red-500">
            {error}
          </p>
        )}
        <div className="save-actions">
          <Button type="submit" disabled={busy || prompt === null}>
            {t("hushtype.save")}
          </Button>
          {saved && (
            <p role="status" className="text-sm">
              {t("hushtype.saved")}
            </p>
          )}
        </div>
      </form>
      <DictationTimings />
    </div>
  );
}
