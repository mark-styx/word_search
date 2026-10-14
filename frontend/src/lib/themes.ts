import type { WordTheme } from "@/types";
import { darken, lighten, normalizeHexColor, withAlpha } from "./colors";

export type VisualTheme = {
  id: string;
  name: string;
  colors: {
    bg: string;
    bgSecondary: string;
    text: string;
    textSecondary: string;
    primary: string;
    primaryHover: string;
    accent: string;
    boardBg: string;
    cellBg: string;
    cellSelected: string;
    cellFound: string;
    border: string;
    success: string;
    error: string;
  };
};

export type UserPreferences = {
  visualTheme: string;
  highlightColor: string;
  preferredWordTheme: WordTheme;
};

export const themes: VisualTheme[] = [
  {
    id: "default",
    name: "Light",
    colors: {
      bg: "#ffffff",
      bgSecondary: "#f3f4f6",
      text: "#111827",
      textSecondary: "#6b7280",
      primary: "#3b82f6",
      primaryHover: "#2563eb",
      accent: "#8b5cf6",
      boardBg: "#f9fafb",
      cellBg: "#ffffff",
      cellSelected: "#3b82f6",
      cellFound: "#10b981",
      border: "#e5e7eb",
      success: "#10b981",
      error: "#ef4444",
    },
  },
  {
    id: "dark",
    name: "Dark",
    colors: {
      bg: "#0f172a",
      bgSecondary: "#1e293b",
      text: "#f1f5f9",
      textSecondary: "#94a3b8",
      primary: "#3b82f6",
      primaryHover: "#60a5fa",
      accent: "#a78bfa",
      boardBg: "#1e293b",
      cellBg: "#334155",
      cellSelected: "#3b82f6",
      cellFound: "#10b981",
      border: "#334155",
      success: "#10b981",
      error: "#f87171",
    },
  },
  {
    id: "ocean",
    name: "Ocean",
    colors: {
      bg: "#0c4a6e",
      bgSecondary: "#075985",
      text: "#e0f2fe",
      textSecondary: "#7dd3fc",
      primary: "#38bdf8",
      primaryHover: "#7dd3fc",
      accent: "#06b6d4",
      boardBg: "#075985",
      cellBg: "#0c4a6e",
      cellSelected: "#38bdf8",
      cellFound: "#2dd4bf",
      border: "#0284c7",
      success: "#2dd4bf",
      error: "#fb7185",
    },
  },
  {
    id: "forest",
    name: "Forest",
    colors: {
      bg: "#052e16",
      bgSecondary: "#14532d",
      text: "#dcfce7",
      textSecondary: "#86efac",
      primary: "#22c55e",
      primaryHover: "#4ade80",
      accent: "#a3e635",
      boardBg: "#14532d",
      cellBg: "#052e16",
      cellSelected: "#22c55e",
      cellFound: "#facc15",
      border: "#166534",
      success: "#4ade80",
      error: "#f87171",
    },
  },
  {
    id: "sunset",
    name: "Sunset",
    colors: {
      bg: "#431407",
      bgSecondary: "#7c2d12",
      text: "#fff7ed",
      textSecondary: "#fdba74",
      primary: "#f97316",
      primaryHover: "#fb923c",
      accent: "#f43f5e",
      boardBg: "#7c2d12",
      cellBg: "#431407",
      cellSelected: "#f97316",
      cellFound: "#fbbf24",
      border: "#9a3412",
      success: "#fbbf24",
      error: "#f43f5e",
    },
  },
  {
    id: "neon",
    name: "Neon",
    colors: {
      bg: "#0a0a0a",
      bgSecondary: "#171717",
      text: "#f5f5f5",
      textSecondary: "#a3a3a3",
      primary: "#e879f9",
      primaryHover: "#f0abfc",
      accent: "#22d3ee",
      boardBg: "#171717",
      cellBg: "#262626",
      cellSelected: "#e879f9",
      cellFound: "#34d399",
      border: "#404040",
      success: "#34d399",
      error: "#f87171",
    },
  },
  {
    id: "retro",
    name: "Retro",
    colors: {
      bg: "#fef3c7",
      bgSecondary: "#fde68a",
      text: "#292524",
      textSecondary: "#78716c",
      primary: "#dc2626",
      primaryHover: "#ef4444",
      accent: "#7c3aed",
      boardBg: "#fef9c3",
      cellBg: "#fef3c7",
      cellSelected: "#dc2626",
      cellFound: "#16a34a",
      border: "#d6d3d1",
      success: "#16a34a",
      error: "#dc2626",
    },
  },
  {
    id: "citrus",
    name: "Citrus",
    colors: {
      bg: "#fff7d6",
      bgSecondary: "#ffe9a8",
      text: "#3f2d15",
      textSecondary: "#8a5a1f",
      primary: "#f59e0b",
      primaryHover: "#fbbf24",
      accent: "#84cc16",
      boardBg: "#fff1bf",
      cellBg: "#fff8e1",
      cellSelected: "#f59e0b",
      cellFound: "#84cc16",
      border: "#f3d27a",
      success: "#65a30d",
      error: "#dc2626",
    },
  },
  {
    id: "berry",
    name: "Berry",
    colors: {
      bg: "#fff1f8",
      bgSecondary: "#ffd7e9",
      text: "#4a1832",
      textSecondary: "#9d4d74",
      primary: "#db2777",
      primaryHover: "#ec4899",
      accent: "#8b5cf6",
      boardBg: "#ffe4f1",
      cellBg: "#fff7fb",
      cellSelected: "#db2777",
      cellFound: "#7c3aed",
      border: "#f4b7d4",
      success: "#16a34a",
      error: "#e11d48",
    },
  },
  {
    id: "paper",
    name: "Paper",
    colors: {
      bg: "#f7f3ea",
      bgSecondary: "#ece3d2",
      text: "#2b2620",
      textSecondary: "#776a5b",
      primary: "#8b5e3c",
      primaryHover: "#a8754a",
      accent: "#0f766e",
      boardBg: "#f1e8d8",
      cellBg: "#fbf8f1",
      cellSelected: "#8b5e3c",
      cellFound: "#0f766e",
      border: "#d8ccb8",
      success: "#15803d",
      error: "#b91c1c",
    },
  },
  {
    id: "midnight",
    name: "Midnight",
    colors: {
      bg: "#08111f",
      bgSecondary: "#122033",
      text: "#eef4ff",
      textSecondary: "#91a3be",
      primary: "#38bdf8",
      primaryHover: "#7dd3fc",
      accent: "#f59e0b",
      boardBg: "#10213a",
      cellBg: "#162842",
      cellSelected: "#38bdf8",
      cellFound: "#f59e0b",
      border: "#24364d",
      success: "#22c55e",
      error: "#fb7185",
    },
  },
  {
    id: "aurora",
    name: "Aurora",
    colors: {
      bg: "#041b2d",
      bgSecondary: "#0b3048",
      text: "#e0fbff",
      textSecondary: "#8dd6e3",
      primary: "#14b8a6",
      primaryHover: "#2dd4bf",
      accent: "#a3e635",
      boardBg: "#08253a",
      cellBg: "#103651",
      cellSelected: "#14b8a6",
      cellFound: "#84cc16",
      border: "#1f4d67",
      success: "#84cc16",
      error: "#fb7185",
    },
  },
  {
    id: "harbor",
    name: "Harbor",
    colors: {
      bg: "#eef6fb",
      bgSecondary: "#d8e8f2",
      text: "#173042",
      textSecondary: "#527289",
      primary: "#0f766e",
      primaryHover: "#0d9488",
      accent: "#2563eb",
      boardBg: "#e2f0f7",
      cellBg: "#f8fbfd",
      cellSelected: "#0f766e",
      cellFound: "#2563eb",
      border: "#bdd3e0",
      success: "#15803d",
      error: "#dc2626",
    },
  },
  {
    id: "ember",
    name: "Ember",
    colors: {
      bg: "#1a120f",
      bgSecondary: "#33211b",
      text: "#fff4ee",
      textSecondary: "#d9a68f",
      primary: "#ea580c",
      primaryHover: "#fb923c",
      accent: "#facc15",
      boardBg: "#241814",
      cellBg: "#3a2720",
      cellSelected: "#ea580c",
      cellFound: "#facc15",
      border: "#5a3b31",
      success: "#84cc16",
      error: "#fb7185",
    },
  },
  {
    id: "blossom",
    name: "Blossom",
    colors: {
      bg: "#fff7fb",
      bgSecondary: "#ffe6f2",
      text: "#4b1730",
      textSecondary: "#9f5a7a",
      primary: "#e11d48",
      primaryHover: "#f43f5e",
      accent: "#7c3aed",
      boardBg: "#fff0f6",
      cellBg: "#fffafd",
      cellSelected: "#e11d48",
      cellFound: "#7c3aed",
      border: "#f5c7da",
      success: "#16a34a",
      error: "#be123c",
    },
  },
  {
    id: "graphite",
    name: "Graphite",
    colors: {
      bg: "#111827",
      bgSecondary: "#1f2937",
      text: "#f9fafb",
      textSecondary: "#9ca3af",
      primary: "#f59e0b",
      primaryHover: "#fbbf24",
      accent: "#60a5fa",
      boardBg: "#18212f",
      cellBg: "#243041",
      cellSelected: "#f59e0b",
      cellFound: "#60a5fa",
      border: "#334155",
      success: "#22c55e",
      error: "#f87171",
    },
  },
];

export const highlightSwatches = [
  "#1d4ed8",
  "#2563eb",
  "#0ea5e9",
  "#06b6d4",
  "#0f766e",
  "#059669",
  "#16a34a",
  "#22c55e",
  "#65a30d",
  "#84cc16",
  "#eab308",
  "#ca8a04",
  "#f59e0b",
  "#ea580c",
  "#f97316",
  "#dc2626",
  "#e11d48",
  "#db2777",
  "#c026d3",
  "#7c3aed",
  "#8b5cf6",
  "#0891b2",
  "#4f46e5",
  "#be123c",
  "#854d0e",
  "#334155",
  "#111827",
];

const VALID_WORD_THEMES: WordTheme[] = [
  "animals",
  "cats",
  "boating",
  "camping",
  "recipe_ingredients",
  "sports",
  "food",
  "fishing",
  "pizza",
  "snacks",
  "fast_food",
  "bugs",
  "movies",
  "cars",
  "desert",
  "mountains",
  "ocean",
  "weather",
  "garden",
  "farm",
  "shiny_things",
  "toys",
  "space",
  "science",
  "geography",
  "technology",
  "music",
  "history",
  "home_renovation",
  "luxury_brands",
  "nature",
];

const PREFERENCE_STORAGE_KEY = "wordsearch:user-preferences";
export const DEFAULT_THEME_ID = "default";
export const DEFAULT_WORD_THEME: WordTheme = "science";
export const DEFAULT_HIGHLIGHT_COLOR = themes[0].colors.primary;

export function getTheme(id: string): VisualTheme {
  return themes.find((t) => t.id === id) || themes[0];
}

export function isWordTheme(value: string | null | undefined): value is WordTheme {
  return VALID_WORD_THEMES.includes((value ?? "") as WordTheme);
}

export function applyTheme(theme: VisualTheme) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  Object.entries(theme.colors).forEach(([key, value]) => {
    const cssVar = `--color-${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
    root.style.setProperty(cssVar, value);
  });
}

export function applyHighlightColor(color: string) {
  if (typeof document === "undefined") return;

  const normalized = normalizeHexColor(color, DEFAULT_HIGHLIGHT_COLOR);
  const root = document.documentElement;
  root.style.setProperty("--color-highlight", normalized);
  root.style.setProperty("--color-highlight-soft", withAlpha(lighten(normalized, 0.2), 0.35));
  root.style.setProperty("--color-highlight-strong", darken(normalized, 0.18));
}

function getPreferenceKeys(userId?: string | null) {
  return [
    userId ? `${PREFERENCE_STORAGE_KEY}:${userId}` : null,
    PREFERENCE_STORAGE_KEY,
  ].filter(Boolean) as string[];
}

export function loadUserPreferences(userId?: string | null): UserPreferences {
  const defaults: UserPreferences = {
    visualTheme: DEFAULT_THEME_ID,
    highlightColor: DEFAULT_HIGHLIGHT_COLOR,
    preferredWordTheme: DEFAULT_WORD_THEME,
  };

  if (typeof window === "undefined") {
    return defaults;
  }

  for (const key of getPreferenceKeys(userId)) {
    const raw = localStorage.getItem(key);
    if (!raw) continue;

    try {
      const parsed = JSON.parse(raw) as Partial<UserPreferences>;
      return {
        visualTheme: parsed.visualTheme && themes.some((theme) => theme.id === parsed.visualTheme)
          ? parsed.visualTheme
          : defaults.visualTheme,
        highlightColor: normalizeHexColor(parsed.highlightColor ?? defaults.highlightColor, defaults.highlightColor),
        preferredWordTheme: isWordTheme(parsed.preferredWordTheme)
          ? parsed.preferredWordTheme
          : defaults.preferredWordTheme,
      };
    } catch {
      // Ignore malformed user preference blobs and fall back to defaults.
    }
  }

  return defaults;
}

export function saveUserPreferences(preferences: UserPreferences, userId?: string | null) {
  if (typeof window === "undefined") return;

  const payload = JSON.stringify({
    visualTheme: preferences.visualTheme,
    highlightColor: normalizeHexColor(preferences.highlightColor, DEFAULT_HIGHLIGHT_COLOR),
    preferredWordTheme: preferences.preferredWordTheme,
  });

  for (const key of getPreferenceKeys(userId)) {
    localStorage.setItem(key, payload);
  }
}
