import { useId } from "react";
import {
  APPLE_ICON_RADIUS,
  BRAND_GRADIENT_STOPS,
  BRAND_LOGO_BODY,
  BRAND_LOGO_GEM,
  BRAND_LOGO_LID,
  BRAND_LOGO_VIEWBOX,
  BRAND_MARK_TRANSFORM,
  BRAND_PLATE_FILL,
} from "@/components/brand-logo-assets";
import { cn } from "@/lib/cn";

type BrandLogoVariant = "brand" | "mono";

export function BrandLogo({
  className,
  variant = "brand",
}: {
  className?: string;
  /** brand: white Apple-style plate + blue gradient; mono: transparent + currentColor */
  variant?: BrandLogoVariant;
}) {
  const maskId = useId();
  const gradientId = useId();
  const isBrand = variant === "brand";
  const fill = isBrand ? `url(#${gradientId})` : "currentColor";

  return (
    <svg
      viewBox={`0 0 ${BRAND_LOGO_VIEWBOX} ${BRAND_LOGO_VIEWBOX}`}
      className={cn("shrink-0", className)}
      fill="none"
      aria-hidden="true"
    >
      <defs>
        {isBrand ? (
          <linearGradient
            id={gradientId}
            x1="16"
            y1="5.8"
            x2="16"
            y2="28.2"
            gradientUnits="userSpaceOnUse"
          >
            {BRAND_GRADIENT_STOPS.map((stop) => (
              <stop key={stop.offset} offset={stop.offset} stopColor={stop.color} />
            ))}
          </linearGradient>
        ) : null}
        <mask id={maskId}>
          <rect width={BRAND_LOGO_VIEWBOX} height={BRAND_LOGO_VIEWBOX} fill="white" />
          <path d={BRAND_LOGO_GEM} fill="black" />
        </mask>
      </defs>
      {isBrand ? (
        <rect
          width={BRAND_LOGO_VIEWBOX}
          height={BRAND_LOGO_VIEWBOX}
          rx={APPLE_ICON_RADIUS}
          fill={BRAND_PLATE_FILL}
        />
      ) : null}
      <g
        transform={isBrand ? BRAND_MARK_TRANSFORM : undefined}
        fill={fill}
        mask={`url(#${maskId})`}
      >
        <path d={BRAND_LOGO_LID} />
        <path d={BRAND_LOGO_BODY} />
      </g>
    </svg>
  );
}
