import { useId } from "react";
import { cn } from "@/lib/cn";

type BrandLogoVariant = "mono" | "brand";

export function BrandLogo({
  className,
  variant = "mono",
}: {
  className?: string;
  variant?: BrandLogoVariant;
}) {
  const maskId = useId();
  const gradientId = useId();
  const fill = variant === "brand" ? `url(#${gradientId})` : "currentColor";

  return (
    <svg
      viewBox="0 0 32 32"
      className={cn("shrink-0", className)}
      fill="none"
      aria-hidden="true"
    >
      <defs>
        {variant === "brand" ? (
          <linearGradient
            id={gradientId}
            x1="16"
            y1="5.8"
            x2="16"
            y2="28.2"
            gradientUnits="userSpaceOnUse"
          >
            <stop offset="0%" stopColor="#3b82f6" />
            <stop offset="55%" stopColor="#2563eb" />
            <stop offset="100%" stopColor="#1d4ed8" />
          </linearGradient>
        ) : null}
        <mask id={maskId}>
          <rect width="32" height="32" fill="white" />
          <path d="M16 19.5 L17.55 21.35 L16 23.2 L14.45 21.35 Z" fill="black" />
        </mask>
      </defs>
      <g fill={fill} mask={`url(#${maskId})`}>
        <path d="M4 11.2 C4 8.1 6.6 5.8 9.8 5.8 H22.2 C25.4 5.8 28 8.1 28 11.2 V13.2 H4 Z" />
        <path d="M9.8 14.2 H22.2 V23.8 C22.2 26.4 20.2 28.2 16 28.2 C11.8 28.2 9.8 26.4 9.8 23.8 Z" />
      </g>
    </svg>
  );
}
