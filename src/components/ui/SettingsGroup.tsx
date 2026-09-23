import React from "react";

interface SettingsGroupProps {
  title?: string;
  description?: string;
  children: React.ReactNode;
}

export const SettingsGroup: React.FC<SettingsGroupProps> = ({
  title,
  description,
  children,
}) => {
  return (
    <section className="settings-group space-y-3">
      {title && (
        <div className="group-heading">
          <h2 className="text-sm font-semibold">{title}</h2>
          {description && (
            <p className="text-sm text-mid-gray mt-1 leading-relaxed">
              {description}
            </p>
          )}
        </div>
      )}
      <div className="settings-surface overflow-visible">
        <div className="divide-y divide-mid-gray/20">{children}</div>
      </div>
    </section>
  );
};
