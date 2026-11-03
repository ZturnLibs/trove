import * as React from "react";
import { cn } from "@/lib/cn";

export type CheckboxProps = React.InputHTMLAttributes<HTMLInputElement>;

/** 品牌色的复选框（原生 input + accent 主题色）。 */
export const Checkbox = React.forwardRef<HTMLInputElement, CheckboxProps>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      type="checkbox"
      className={cn(
        "size-3.5 shrink-0 cursor-pointer accent-[var(--accent)]",
        className,
      )}
      {...props}
    />
  ),
);
Checkbox.displayName = "Checkbox";
