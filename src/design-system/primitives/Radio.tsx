import * as React from "react";
import { cn } from "@/lib/cn";

export type RadioProps = React.InputHTMLAttributes<HTMLInputElement>;

/** 品牌色的单选框（原生 input + accent 主题色）。 */
export const Radio = React.forwardRef<HTMLInputElement, RadioProps>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      type="radio"
      className={cn(
        "size-3.5 shrink-0 cursor-pointer accent-[var(--accent)]",
        className,
      )}
      {...props}
    />
  ),
);
Radio.displayName = "Radio";
