import * as React from "react";
import { cn } from "@/lib/cn";

export type InputProps = React.InputHTMLAttributes<HTMLInputElement>;

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn(
        "h-8 w-full rounded-[var(--radius-control)] border border-border bg-surface-raised px-2.5 text-[13px] text-foreground placeholder:text-muted outline-none focus:ring-2 focus:ring-accent/35",
        className,
      )}
      {...props}
    />
  ),
);
Input.displayName = "Input";
