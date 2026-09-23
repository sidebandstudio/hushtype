import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { commands, type TimingRecord } from "@/bindings";
import { SettingsGroup } from "@/components/ui/SettingsGroup";

export function DictationTimings() {
  const { t } = useTranslation();
  const [rows, setRows] = useState<TimingRecord[]>([]);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    const unlisten = listen<TimingRecord>("dictation-timing", ({ payload }) => {
      if (active)
        setRows((current) =>
          [
            payload,
            ...current.filter((r) => r.timestamp !== payload.timestamp),
          ].slice(0, 20),
        );
    });
    commands
      .getDictationTimings()
      .then((result) => {
        if (active) setRows(result);
      })
      .catch((e: unknown) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
      void unlisten.then((fn) => fn());
    };
  }, []);
  return (
    <SettingsGroup
      title={t("hushtype.timings.title")}
      description={t("hushtype.timings.description")}
    >
      <div className="p-4 overflow-x-auto">
        {error && <p role="alert">{error}</p>}
        {rows.length === 0 ? (
          <p className="text-sm text-mid-gray">{t("hushtype.timings.empty")}</p>
        ) : (
          <table className="w-full text-xs text-start tabular-nums">
            <thead>
              <tr>
                {[
                  "time",
                  "mode",
                  "capture",
                  "stt",
                  "refine",
                  "paste",
                  "total",
                  "outcome",
                ].map((key) => (
                  <th
                    key={key}
                    scope="col"
                    className="text-start p-2 font-medium"
                  >
                    {t(`hushtype.timings.${key}`)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.timestamp} className="border-t border-mid-gray/20">
                  <td className="p-2 whitespace-nowrap">
                    {new Date(row.timestamp).toLocaleTimeString()}
                  </td>
                  <td className="p-2">{row.mode}</td>
                  {[
                    row.capture_stop_ms,
                    row.stt_ms,
                    row.refine_ms,
                    row.paste_ms,
                    row.total_ms,
                  ].map((value, index) => (
                    <td key={index} className="p-2">
                      {value}
                    </td>
                  ))}
                  <td className="p-2 font-mono">
                    {row.outcome}
                    <br />
                    {row.paste_outcome}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </SettingsGroup>
  );
}
