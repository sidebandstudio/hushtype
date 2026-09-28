import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type Mode,
  type ContextFields,
  type Style,
} from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import { SettingsGroup } from "@/components/ui/SettingsGroup";
import { Input } from "@/components/ui/Input";
import { Textarea } from "@/components/ui/Textarea";
import { ToggleSwitch } from "@/components/ui/ToggleSwitch";
import { Button } from "@/components/ui/Button";
import { ChevronDown } from "lucide-react";

const contextDefaults: ContextFields = {
  app: true,
  window_title: true,
  url: true,
  selected_text: false,
  clipboard: false,
};
const styles: Style[] = ["serious", "casual", "very_casual"];
const lines = (text: string) =>
  text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);

export function ModesSettings() {
  const { t } = useTranslation();
  const { settings, refreshSettings } = useSettings();
  const [modes, setModes] = useState<Mode[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  const [expanded, setExpanded] = useState<string | null>(null);
  useEffect(() => {
    if (settings?.modes) setModes(settings.modes);
  }, [settings?.modes]);
  function update(index: number, change: Partial<Mode>) {
    setModes(
      modes.map((mode, i) => (i === index ? { ...mode, ...change } : mode)),
    );
    setSaved(false);
  }
  function move(index: number, offset: number) {
    const next = [...modes];
    [next[index], next[index + offset]] = [next[index + offset], next[index]];
    setModes(next);
    setSaved(false);
  }
  async function save(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError("");
    setSaved(false);
    try {
      const result = await commands.updateModes(modes);
      if (result.status === "error") throw new Error(result.error);
      await refreshSettings();
      setSaved(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <form onSubmit={save} className="max-w-3xl w-full mx-auto space-y-6">
      <p className="text-sm text-mid-gray">{t("hushtype.modes.description")}</p>
      {modes.map((mode, index) => (
        <SettingsGroup key={mode.id}>
          <details open={mode.id === expanded}>
            <summary
              className="mode-summary"
              onClick={(event) => {
                event.preventDefault();
                setExpanded(expanded === mode.id ? null : mode.id);
              }}
            >
              <span className="mode-number" aria-hidden="true">
                {String(index + 1).padStart(2, "0")}
              </span>
              <span className="mode-label">
                <strong>{mode.name || t("hushtype.modes.unnamed")}</strong>
                <small>
                  {t("hushtype.ui.modeSummary", {
                    matchers:
                      mode.id === "default"
                        ? t("hushtype.ui.modeFallback")
                        : t("hushtype.ui.modeMatchers", {
                            count:
                              (mode.bundle_ids?.length ?? 0) +
                              (mode.url_globs?.length ?? 0),
                          }),
                    style: t(
                      `hushtype.modes.styles.${mode.style ?? "serious"}.label`,
                    ),
                  })}
                </small>
              </span>
              <span
                className={`mode-status ${mode.refine_enabled ? "is-on" : ""}`}
              >
                {t(
                  mode.refine_enabled
                    ? "hushtype.ui.modeOn"
                    : "hushtype.ui.modeOff",
                )}
              </span>
              <ChevronDown size={16} aria-hidden="true" />
            </summary>
            <div className="mode-editor space-y-4">
              {mode.id !== "default" && (
                <>
                  <label className="grid gap-1 text-sm">
                    {t("hushtype.modes.name")}
                    <Input
                      value={mode.name}
                      maxLength={80}
                      required
                      disabled={busy}
                      onChange={(e) => update(index, { name: e.target.value })}
                    />
                  </label>
                  <div className="flex gap-2">
                    <Button
                      type="button"
                      variant="secondary"
                      disabled={busy || index === 0}
                      onClick={() => move(index, -1)}
                    >
                      {t("hushtype.modes.up")}
                    </Button>
                    <Button
                      type="button"
                      variant="secondary"
                      disabled={busy || index >= modes.length - 2}
                      onClick={() => move(index, 1)}
                    >
                      {t("hushtype.modes.down")}
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      disabled={busy}
                      onClick={() => {
                        setModes(modes.filter((m) => m.id !== mode.id));
                        setSaved(false);
                      }}
                    >
                      {t("hushtype.remove")}
                    </Button>
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    {(["bundle_ids", "url_globs"] as const).map((field) => (
                      <label key={field} className="grid gap-1 text-sm">
                        {t(`hushtype.modes.${field}`)}
                        <Textarea
                          rows={4}
                          value={(mode[field] ?? []).join("\n")}
                          disabled={busy}
                          onChange={(e) =>
                            update(index, {
                              [field]: e.target.value.split("\n"),
                            })
                          }
                          onBlur={(e) =>
                            update(index, { [field]: lines(e.target.value) })
                          }
                        />
                      </label>
                    ))}
                  </div>
                </>
              )}
              <fieldset className="space-y-2">
                <legend className="text-sm">{t("hushtype.modes.style")}</legend>
                <p className="text-xs text-mid-gray">
                  {t("hushtype.modes.styleDescription")}
                </p>
                <div className="style-options">
                  {styles.map((style) => (
                    <label key={style} className="style-option">
                      <input
                        type="radio"
                        name={`style-${mode.id}`}
                        value={style}
                        checked={(mode.style ?? "serious") === style}
                        disabled={busy}
                        onChange={() => update(index, { style })}
                      />
                      <strong>
                        {t(`hushtype.modes.styles.${style}.label`)}
                      </strong>
                      <small>
                        {t(`hushtype.modes.styles.${style}.detail`)}
                      </small>
                      <span className="style-example">
                        {t(`hushtype.modes.styles.${style}.example`)}
                      </span>
                    </label>
                  ))}
                </div>
              </fieldset>
              <ToggleSwitch
                label={t("hushtype.modes.refine")}
                description={t("hushtype.modes.refineDescription")}
                descriptionMode="inline"
                checked={mode.refine_enabled}
                onChange={(refine_enabled) => update(index, { refine_enabled })}
                disabled={busy}
              />
              <label className="grid gap-1 text-sm">
                {t("hushtype.modes.promptBehavior")}
                <select
                  className="field-input px-3 py-2 rounded-lg"
                  value={mode.prompt_behavior ?? "append"}
                  disabled={busy}
                  onChange={(e) =>
                    update(index, {
                      prompt_behavior:
                        e.target.value === "replace" ? "replace" : "append",
                    })
                  }
                >
                  <option value="append">{t("hushtype.modes.append")}</option>
                  <option value="replace">{t("hushtype.modes.replace")}</option>
                </select>
              </label>
              <label className="grid gap-1 text-sm">
                {t("hushtype.modes.prompt")}
                <Textarea
                  value={mode.prompt ?? ""}
                  rows={4}
                  disabled={busy}
                  required={mode.prompt_behavior === "replace"}
                  onChange={(e) => update(index, { prompt: e.target.value })}
                />
              </label>
              <fieldset className="mode-context space-y-3">
                <legend className="text-sm mb-2">
                  {t("hushtype.modes.context")}
                </legend>
                {(Object.keys(contextDefaults) as (keyof ContextFields)[]).map(
                  (field) => (
                    <label
                      key={field}
                      className="flex gap-2 items-center text-sm"
                    >
                      <input
                        type="checkbox"
                        checked={
                          mode.context?.[field] ?? contextDefaults[field]
                        }
                        disabled={busy}
                        onChange={(e) =>
                          update(index, {
                            context: {
                              ...contextDefaults,
                              ...mode.context,
                              [field]: e.target.checked,
                            },
                          })
                        }
                      />
                      {t(`hushtype.modes.contextFields.${field}`)}
                    </label>
                  ),
                )}
              </fieldset>
            </div>
          </details>
        </SettingsGroup>
      ))}
      <Button
        type="button"
        variant="secondary"
        disabled={busy || !settings || modes.length >= 32}
        onClick={() => {
          const added: Mode = {
            id: crypto.randomUUID(),
            name: t("hushtype.modes.unnamed"),
            bundle_ids: [],
            url_globs: [],
            refine_enabled: true,
            prompt: "",
            prompt_behavior: "append",
            context: { ...contextDefaults },
            style: "serious",
          };
          setModes([...modes.slice(0, -1), added, ...modes.slice(-1)]);
          setExpanded(added.id);
          setSaved(false);
        }}
      >
        {t("hushtype.modes.add")}
      </Button>
      {error && (
        <p role="alert" className="text-sm text-red-500">
          {error}
        </p>
      )}
      <div className="save-actions">
        <Button type="submit" disabled={busy || modes.length === 0}>
          {t("hushtype.save")}
        </Button>
        {saved && (
          <p role="status" className="text-sm">
            {t("hushtype.saved")}
          </p>
        )}
      </div>
    </form>
  );
}
