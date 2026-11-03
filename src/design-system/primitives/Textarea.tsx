import * as React from "react";
import { cn } from "@/lib/cn";

export type TextareaProps = React.TextareaHTMLAttributes<HTMLTextAreaElement>;

export const Textarea = React.forwardRef<
  HTMLTextAreaElement,
  TextareaProps
>(({ className, ...props }, ref) => (
  <textarea
    ref={ref}
    className={cn(
      "w-full rounded-[var(--radius-control)] border border-border bg-surface-raised px-2.5 py-1.5 text-[13px] text-foreground placeholder:text-muted outline-none focus:ring-2 focus:ring-accent/35",
      className,
    )}
    {...props}
  />
));
Textarea.displayName = "Textarea";
