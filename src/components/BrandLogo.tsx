import { useId } from "react";
import { cn } from "@/lib/cn";

export function BrandLogo({ className }: { className?: string }) {
  const maskId = useId();
  return (
    <svg
      viewBox="0 0 32 32"
      className={cn("shrink-0", className)}
      aria-hidden="true"
    >
      <defs>
        <mask id={maskId}>
          <rect width="32" height="32" fill="white" />
          <circle cx="16" cy="21.3" r="2.2" fill="black" />
        </mask>
      </defs>
      <g fill="currentColor" mask={`url(#${maskId})`}>
        <path d="M4.5 10 a3 3 0 0 1 3 -3 h17 a3 3 0 0 1 3 3 v6 H4.5 z" />
        <path d="M10 18 h12 v7 a2 2 0 0 1 -2 2 h-8 a2 2 0 0 1 -2 -2 z" />
      </g>
    </svg>
  );
}
