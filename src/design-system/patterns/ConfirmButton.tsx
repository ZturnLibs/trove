import { useEffect, useRef, useState, type ReactNode } from "react";
import { Button, type ButtonProps } from "@/design-system/primitives/Button";

type ConfirmButtonProps = Omit<ButtonProps, "onClick"> & {
  confirmLabel: ReactNode;
  onConfirm: () => void;
  /** Tooltip shown while armed. Defaults to confirmLabel. */
  confirmTitle?: string;
  /** Reset the armed state whenever this value changes (e.g. selected item id). */
  resetKey?: unknown;
  /** Variant used while armed. Defaults to "danger". */
  confirmVariant?: ButtonProps["variant"];
};

export function ConfirmButton({
  confirmLabel,
  onConfirm,
  confirmTitle,
  resetKey,
  confirmVariant = "danger",
  children,
  disabled,
  variant,
  ...rest
}: ConfirmButtonProps) {
  const [armed, setArmed] = useState(false);
  const timer = useRef<number | null>(null);
  const baseVariant = variant ?? "ghost";

  useEffect(() => {
    setArmed(false);
    if (timer.current) {
      window.clearTimeout(timer.current);
      timer.current = null;
    }
  }, [resetKey]);

  useEffect(
    () => () => {
      if (timer.current) window.clearTimeout(timer.current);
    },
    [],
  );

  const handleClick = () => {
    if (disabled) return;
    if (armed) {
      if (timer.current) window.clearTimeout(timer.current);
      timer.current = null;
      setArmed(false);
      onConfirm();
    } else {
      setArmed(true);
      timer.current = window.setTimeout(() => setArmed(false), 3000);
    }
  };

  return (
    <Button
      {...rest}
      title={armed ? confirmTitle ?? String(confirmLabel) : rest.title}
      variant={armed ? confirmVariant : baseVariant}
      disabled={disabled}
      onClick={handleClick}
    >
      {armed ? confirmLabel : children}
    </Button>
  );
}
