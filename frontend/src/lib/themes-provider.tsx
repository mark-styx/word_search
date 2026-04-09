"use client";

import React, { createContext, useContext, useEffect, useState } from "react";
import { getTheme, getSavedThemeId, saveThemeId, applyTheme, type VisualTheme } from "./themes";

interface ThemeContextValue {
  theme: VisualTheme;
  setTheme: (id: string) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<VisualTheme>(() => getTheme(getSavedThemeId()));

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  const setTheme = (id: string) => {
    const t = getTheme(id);
    setThemeState(t);
    saveThemeId(id);
  };

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}
