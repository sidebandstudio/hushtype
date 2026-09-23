import { useTranslation } from "react-i18next";

const HushtypeWordmark = ({
  width = 180,
  className = "",
}: {
  width?: number;
  className?: string;
}) => {
  const { t } = useTranslation();
  return (
    <span className={`brand-wordmark ${className}`} style={{ width }}>
      <span className="brand-symbol" aria-hidden="true" />
      <span>{t("hushtype.name")}</span>
    </span>
  );
};

export default HushtypeWordmark;
