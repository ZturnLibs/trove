/** Shared Trove logo geometry — keep in sync with design/logo.svg & design/logo-tray.svg */

export const BRAND_LOGO_VIEWBOX = 32;

/** Apple app-icon corner radius ratio (~22.37% of canvas edge). */
export const APPLE_ICON_RADIUS_RATIO = 0.2237;

export const APPLE_ICON_RADIUS = BRAND_LOGO_VIEWBOX * APPLE_ICON_RADIUS_RATIO;

/** Inset scale for the mark on the white plate. */
export const BRAND_MARK_SCALE = 0.875;

export const BRAND_MARK_TRANSFORM = `translate(16 16) scale(${BRAND_MARK_SCALE}) translate(-16 -16)`;

export const BRAND_PLATE_FILL = "#ffffff";

export const BRAND_LOGO_LID =
  "M4 11.2 C4 8.1 6.6 5.8 9.8 5.8 H22.2 C25.4 5.8 28 8.1 28 11.2 V13.2 H4 Z";

export const BRAND_LOGO_BODY =
  "M9.8 14.2 H22.2 V23.8 C22.2 26.4 20.2 28.2 16 28.2 C11.8 28.2 9.8 26.4 9.8 23.8 Z";

export const BRAND_LOGO_GEM =
  "M16 19.5 L17.55 21.35 L16 23.2 L14.45 21.35 Z";

export const BRAND_GRADIENT_STOPS = [
  { offset: "0%", color: "#3b82f6" },
  { offset: "55%", color: "#2563eb" },
  { offset: "100%", color: "#1d4ed8" },
] as const;
