import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { RefreshCw } from "lucide-react";
import { useModelStore } from "../../stores/modelStore";
import { Button } from "../ui/Button";
import ModelCard from "./ModelCard";
import { SetupLayout } from "./SetupLayout";

interface OnboardingProps {
  onModelSelected: () => void;
  preview?: boolean;
}

export default function Onboarding({
  onModelSelected,
  preview = false,
}: OnboardingProps) {
  const { t } = useTranslation();
  const { models, rescanLocalModels, isRescanning, selectModel } =
    useModelStore();
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const installed = models.filter((model) => model.is_downloaded);

  async function select(id: string) {
    if (preview || selectedModelId) return;
    setSelectedModelId(id);
    try {
      if (await selectModel(id)) onModelSelected();
      else toast.error(t("onboarding.errors.selectModel"));
    } finally {
      setSelectedModelId(null);
    }
  }

  return (
    <SetupLayout
      step={2}
      title={t("hushtype.ui.chooseModel")}
      description={t("hushtype.ui.modelDescription")}
    >
      <div className="space-y-4">
        {installed.map((model) => (
          <ModelCard
            key={model.id}
            model={model}
            status={selectedModelId === model.id ? "switching" : "available"}
            disabled={selectedModelId !== null || preview}
            onSelect={select}
            showRecommended={false}
          />
        ))}
        {installed.length === 0 && (
          <p className="text-sm text-mid-gray">{t("hushtype.localModels")}</p>
        )}
        <Button
          variant="secondary"
          disabled={preview || isRescanning || selectedModelId !== null}
          onClick={() => rescanLocalModels()}
        >
          <span className="flex items-center gap-2">
            <RefreshCw size={14} aria-hidden="true" />
            {t("hushtype.refresh")}
          </span>
        </Button>
      </div>
    </SetupLayout>
  );
}
