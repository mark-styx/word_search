type RgbColor = {
  r: number;
  g: number;
  b: number;
};

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

export function normalizeHexColor(value: string, fallback = "#3b82f6") {
  const trimmed = value.trim();
  if (/^#[0-9a-fA-F]{6}$/.test(trimmed)) {
    return trimmed.toLowerCase();
  }
  if (/^#[0-9a-fA-F]{3}$/.test(trimmed)) {
    return `#${trimmed[1]}${trimmed[1]}${trimmed[2]}${trimmed[2]}${trimmed[3]}${trimmed[3]}`.toLowerCase();
  }
  return fallback;
}

function hexToRgb(value: string): RgbColor {
  const normalized = normalizeHexColor(value);
  const hex = normalized.slice(1);
  return {
    r: Number.parseInt(hex.slice(0, 2), 16),
    g: Number.parseInt(hex.slice(2, 4), 16),
    b: Number.parseInt(hex.slice(4, 6), 16),
  };
}

function rgbToHex({ r, g, b }: RgbColor) {
  const toHex = (channel: number) => clamp(Math.round(channel), 0, 255).toString(16).padStart(2, "0");
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
}

export function mixColors(base: string, target: string, amount: number) {
  const safeAmount = clamp(amount, 0, 1);
  const start = hexToRgb(base);
  const end = hexToRgb(target);
  return rgbToHex({
    r: start.r + (end.r - start.r) * safeAmount,
    g: start.g + (end.g - start.g) * safeAmount,
    b: start.b + (end.b - start.b) * safeAmount,
  });
}

export function lighten(color: string, amount: number) {
  return mixColors(color, "#ffffff", amount);
}

export function darken(color: string, amount: number) {
  return mixColors(color, "#000000", amount);
}

export function withAlpha(color: string, alpha: number) {
  const { r, g, b } = hexToRgb(color);
  return `rgba(${r}, ${g}, ${b}, ${clamp(alpha, 0, 1)})`;
}

export function readableTextColor(background: string) {
  const { r, g, b } = hexToRgb(background);
  const luminance = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
  return luminance > 0.55 ? "#0f172a" : "#ffffff";
}
