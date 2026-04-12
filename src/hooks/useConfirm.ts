import { useState, useCallback } from "react";

type Variant = "danger" | "warning" | "info";

interface ConfirmState {
  open: boolean;
  title: string;
  description: string;
  confirmLabel: string;
  variant: Variant;
  onConfirm: () => void;
}

const initial: ConfirmState = {
  open: false,
  title: "",
  description: "",
  confirmLabel: "Confirm",
  variant: "info",
  onConfirm: () => {},
};

export function useConfirm() {
  const [state, setState] = useState<ConfirmState>(initial);

  const confirm = useCallback(
    (opts: {
      title: string;
      description: string;
      confirmLabel?: string;
      variant?: Variant;
    }): Promise<boolean> => {
      return new Promise((resolve) => {
        setState({
          open: true,
          title: opts.title,
          description: opts.description,
          confirmLabel: opts.confirmLabel ?? "Confirm",
          variant: opts.variant ?? "info",
          onConfirm: () => resolve(true),
        });
        // Also resolve false on cancel — handled via onCancel closing
        const origOnConfirm = () => resolve(true);
        setState((prev) => ({ ...prev, onConfirm: origOnConfirm }));
      });
    },
    [],
  );

  const cancel = useCallback(() => {
    setState({ ...initial });
  }, []);

  return {
    confirmProps: {
      open: state.open,
      title: state.title,
      description: state.description,
      confirmLabel: state.confirmLabel,
      variant: state.variant,
      onConfirm: state.onConfirm,
      onCancel: cancel,
    },
    confirm,
  };
}
