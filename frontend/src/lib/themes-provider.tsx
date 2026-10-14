"use client";

import React, { createContext, useContext, useEffect, useRef, useState } from "react";
import { usePathname } from "next/navigation";
import { api, getAccessToken, getCurrentUserId } from "./api";
import { normalizeHexColor } from "./colors";
import { rememberCurrentSession } from "./profiles";
import {
  DEFAULT_HIGHLIGHT_COLOR,
  DEFAULT_WORD_THEME,
  applyHighlightColor,
  applyTheme,
  getTheme,
  isWordTheme,
  loadUserPreferences,
  saveUserPreferences,
  type UserPreferences,
  type VisualTheme,
} from "./themes";
import type { WordTheme } from "@/types";

interface ThemeContextValue {
  theme: VisualTheme;
  setTheme: (id: string) => void;
  highlightColor: string;
  setHighlightColor: (color: string) => void;
  preferredWordTheme: WordTheme;
  setPreferredWordTheme: (theme: WordTheme) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const [theme, setThemeState] = useState<VisualTheme>(() => getTheme("default"));
  const [highlightColor, setHighlightColorState] = useState(DEFAULT_HIGHLIGHT_COLOR);
  const [preferredWordTheme, setPreferredWordThemeState] = useState<WordTheme>(DEFAULT_WORD_THEME);
  const [hydrated, setHydrated] = useState(false);
  const [syncReady, setSyncReady] = useState(false);
  const lastSyncedPreferences = useRef<string | null>(null);

  useEffect(() => {
    setSyncReady(false);
    const userId = getCurrentUserId();
    const preferences = loadUserPreferences(userId);
    setThemeState(getTheme(preferences.visualTheme));
    setHighlightColorState(preferences.highlightColor);
    setPreferredWordThemeState(preferences.preferredWordTheme);
    setHydrated(true);

    if (!getAccessToken()) {
      setSyncReady(true);
      return;
    }

    let cancelled = false;
    void api.getMe().then((user) => {
      if (cancelled) return;

      const nextPreferences: UserPreferences = {
        visualTheme: user.visual_theme && user.visual_theme.trim()
          ? user.visual_theme
          : preferences.visualTheme,
        highlightColor: normalizeHexColor(
          user.highlight_color ?? preferences.highlightColor,
          preferences.highlightColor
        ),
        preferredWordTheme: isWordTheme(user.preferred_word_theme)
          ? user.preferred_word_theme
          : preferences.preferredWordTheme,
      };

      setThemeState(getTheme(nextPreferences.visualTheme));
      setHighlightColorState(nextPreferences.highlightColor);
      setPreferredWordThemeState(nextPreferences.preferredWordTheme);
      saveUserPreferences(nextPreferences, userId);
      rememberCurrentSession(user);
    }).catch(() => {
      // Ignore preference sync failures and keep local values.
    }).finally(() => {
      if (!cancelled) {
        setSyncReady(true);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [pathname]);

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  useEffect(() => {
    applyHighlightColor(highlightColor);
  }, [highlightColor]);

  useEffect(() => {
    if (!hydrated || !syncReady) return;

    const preferences: UserPreferences = {
      visualTheme: theme.id,
      highlightColor,
      preferredWordTheme,
    };
    const userId = getCurrentUserId();
    saveUserPreferences(preferences, userId);

    if (!getAccessToken()) return;

    const payload = JSON.stringify(preferences);
    if (lastSyncedPreferences.current === payload) return;
    lastSyncedPreferences.current = payload;

    void api.updatePreferences({
      visual_theme: preferences.visualTheme,
      preferred_word_theme: preferences.preferredWordTheme,
      highlight_color: preferences.highlightColor,
    }).catch(() => {
      lastSyncedPreferences.current = null;
    });
  }, [theme.id, highlightColor, preferredWordTheme, hydrated, syncReady]);

  const setTheme = (id: string) => {
    setThemeState(getTheme(id));
  };

  return (
    <ThemeContext.Provider
      value={{
        theme,
        setTheme,
        highlightColor,
        setHighlightColor: setHighlightColorState,
        preferredWordTheme,
        setPreferredWordTheme: setPreferredWordThemeState,
      }}
    >
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}
