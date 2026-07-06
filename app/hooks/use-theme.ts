"use client";

import { useCallback, useState } from "react";
import {
  BUILTIN_THEMES,
  CUSTOM_STORAGE_KEY,
  THEME_STORAGE_KEY,
  ThemeTokens,
  applyTheme,
  loadCustomTokens,
  resolveTokens,
} from "@/lib/themes";

/**
 * Theme state hook. The <head> init script already applied the saved theme on
 * launch; this hook mirrors that state for the UI and handles live switching.
 */
export function useTheme() {
  // Lazy init from storage. The <head> script already applied the theme; here
  // we only mirror the persisted selection for the UI. Guards keep static-export
  // prerender (no localStorage) safe.
  const [themeId, setThemeIdState] = useState<string>(() =>
    typeof localStorage === "undefined"
      ? "default"
      : localStorage.getItem(THEME_STORAGE_KEY) || "default",
  );
  const [customTokens, setCustomTokensState] = useState<ThemeTokens>(() =>
    loadCustomTokens(),
  );

  const setThemeId = useCallback((id: string) => {
    setThemeIdState(id);
    localStorage.setItem(THEME_STORAGE_KEY, id);
    applyTheme(resolveTokens(id, loadCustomTokens()));
  }, []);

  const setCustomTokens = useCallback((tokens: ThemeTokens) => {
    setCustomTokensState(tokens);
    localStorage.setItem(CUSTOM_STORAGE_KEY, JSON.stringify(tokens));
    // Apply live only when the custom theme is the active selection.
    if (localStorage.getItem(THEME_STORAGE_KEY) === "custom") {
      applyTheme(tokens);
    }
  }, []);

  return {
    themeId,
    setThemeId,
    customTokens,
    setCustomTokens,
    themes: BUILTIN_THEMES,
  };
}
