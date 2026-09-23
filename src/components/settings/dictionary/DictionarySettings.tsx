import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type Replacement } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import { SettingsGroup } from "@/components/ui/SettingsGroup";
import { Input } from "@/components/ui/Input";
import { Button } from "@/components/ui/Button";
import { CustomWords } from "../CustomWords";

export function DictionarySettings() {
  const { t } = useTranslation();
  const { settings, refreshSettings } = useSettings();
  const [rows, setRows] = useState<Replacement[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  useEffect(() => {
    if (settings) setRows(settings.replacements ?? []);
  }, [settings?.replacements]);

  async function save(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError("");
    setSaved(false);
    try {
      const result = await commands.updateReplacements(rows);
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
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup
        title={t("hushtype.dictionary.terms")}
        description={t("hushtype.dictionary.description")}
      >
        <CustomWords grouped descriptionMode="inline" />
      </SettingsGroup>
      <form
        onSubmit={save}
        onChange={() => setSaved(false)}
        className="space-y-4"
      >
        <SettingsGroup
          title={t("hushtype.dictionary.replacements")}
          description={t("hushtype.dictionary.replacementDescription")}
        >
          {rows.length === 0 && (
            <p className="empty-replacements">
              {t("hushtype.ui.noReplacements")}
            </p>
          )}
          {rows.map((row, index) => (
            <div key={index} className="p-4 flex items-end gap-2">
              {(["from", "to"] as const).map((field) => (
                <label
                  key={field}
                  className="grid gap-1 flex-1 min-w-0 text-sm"
                >
                  {t(`hushtype.dictionary.${field}`)}
                  <Input
                    value={row[field]}
                    required={field === "from"}
                    maxLength={200}
                    disabled={busy}
                    onChange={(e) =>
                      setRows(
                        rows.map((r, i) =>
                          i === index ? { ...r, [field]: e.target.value } : r,
                        ),
                      )
                    }
                  />
                </label>
              ))}
              <Button
                type="button"
                variant="ghost"
                disabled={busy}
                aria-label={t("hushtype.dictionary.removeRow", {
                  number: index + 1,
                })}
                onClick={() => {
                  setRows(rows.filter((_, i) => i !== index));
                  setSaved(false);
                }}
              >
                {t("hushtype.remove")}
              </Button>
            </div>
          ))}
          <div className="p-4">
            <Button
              type="button"
              variant="secondary"
              disabled={busy || rows.length >= 1000}
              onClick={() => {
                setRows([...rows, { from: "", to: "" }]);
                setSaved(false);
              }}
            >
              {t("hushtype.dictionary.add")}
            </Button>
          </div>
        </SettingsGroup>
        {error && (
          <p role="alert" className="text-sm text-red-500">
            {error}
          </p>
        )}
        <div className="save-actions">
          <Button type="submit" disabled={busy || !settings}>
            {t("hushtype.save")}
          </Button>
          {saved && (
            <p role="status" className="text-sm">
              {t("hushtype.saved")}
            </p>
          )}
        </div>
      </form>
    </div>
  );
}
