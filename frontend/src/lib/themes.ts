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
];

export function getTheme(id: string): VisualTheme {
  return themes.find((t) => t.id === id) || themes[0];
}

export function applyTheme(theme: VisualTheme) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  Object.entries(theme.colors).forEach(([key, value]) => {
    const cssVar = `--color-${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
    root.style.setProperty(cssVar, value);
  });
}

export function getSavedThemeId(): string {
  if (typeof window === "undefined") return "default";
  return localStorage.getItem("visual-theme") || "default";
}

export function saveThemeId(id: string) {
  if (typeof window === "undefined") return;
  localStorage.setItem("visual-theme", id);
}
