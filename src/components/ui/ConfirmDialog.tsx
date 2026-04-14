import { useEffect, useCallback } from "react";
import { AlertTriangle, Info, Trash2, X } from "lucide-react";

type Variant = "danger" | "warning" | "info";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: Variant;
  onConfirm: () => void;
  onCancel: () => void;
}

const variantConfig = {
  danger: {
    icon: Trash2,
    iconBg: "bg-destructive/10",
    iconColor: "text-destructive",
    confirmBtn: "bg-destructive text-destructive-foreground hover:bg-destructive/90",
  },
  warning: {
    icon: AlertTriangle,
    iconBg: "bg-warning/10",
    iconColor: "text-warning",
    confirmBtn: "bg-primary text-primary-foreground hover:bg-primary/90",
  },
  info: {
    icon: Info,
    iconBg: "bg-primary/10",
    iconColor: "text-primary",
    confirmBtn: "bg-primary text-primary-foreground hover:bg-primary/90",
  },
};

export function ConfirmDialog({
  open,
  title,
  description,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  variant = "info",
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
    },
    [onCancel],
  );

  useEffect(() => {
    if (!open) return;
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [open, handleKeyDown]);

  if (!open) return null;

  const config = variantConfig[variant];
  const Icon = config.icon;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
      onClick={(e) => {
        if (e.target === e.currentTarget) onCancel();
      }}
    >
      <div role="dialog" aria-modal="true" aria-label={title} className="bg-card border rounded-xl shadow-2xl shadow-black/30 w-96 overflow-hidden animate-fade-in">
        {/* Header */}
        <div className="flex items-start gap-3 px-5 pt-5 pb-3">
          <div className={`w-9 h-9 rounded-lg ${config.iconBg} flex items-center justify-center shrink-0 mt-0.5`}>
            <Icon size={18} className={config.iconColor} />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-semibold leading-tight">{title}</h3>
            <p className="text-xs text-muted-foreground mt-1.5 leading-relaxed">{description}</p>
          </div>
          <button
            type="button"
            onClick={onCancel}
            title="Close"
            className="p-1 rounded-md text-muted-foreground/50 hover:text-foreground hover:bg-secondary transition-colors shrink-0 -mt-0.5"
          >
            <X size={14} />
          </button>
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-2 px-5 py-3.5 border-t bg-secondary/30">
          <button
            type="button"
            onClick={onCancel}
            className="px-3.5 py-1.5 text-xs font-medium rounded-lg bg-secondary hover:bg-accent transition-colors"
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            onClick={() => {
              onConfirm();
              onCancel();
            }}
            className={`px-3.5 py-1.5 text-xs font-medium rounded-lg transition-colors ${config.confirmBtn}`}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
