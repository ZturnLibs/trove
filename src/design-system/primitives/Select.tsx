import * as React from "react";
import { cn } from "@/lib/cn";

export type SelectProps = React.SelectHTMLAttributes<HTMLSelectElement>;

/** 品牌样式的原生 select：保留平台交互与无障碍语义，统一视觉。 */
export const Select = React.forwardRef<HTMLSelectElement, SelectProps>(
  ({ className, children, ...props }, ref) => (
    <select
      ref={ref}
      className={cn(
        "h-8 rounded-[var(--radius-control)] border border-border bg-surface-raised px-2 text-[13px] text-foreground outline-none focus:ring-2 focus:ring-accent/35 disabled:opacity-50",
        className,
      )}
      {...props}
    >
      {children}
    </select>
  ),
);
Select.displayName = "Select";
