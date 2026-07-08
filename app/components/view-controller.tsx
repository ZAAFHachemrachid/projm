"use client";

import { useEffect } from "react";
import { initZoom, zoomIn, zoomOut, zoomReset } from "@/lib/view-prefs";

/**
 * Applies the saved whole-app zoom on startup and binds the browser-standard
 * zoom shortcuts:
 *   Ctrl/Cmd +   → zoom in
 *   Ctrl/Cmd -   → zoom out
 *   Ctrl/Cmd 0   → reset to 100%
 *
 * Rendered once (in the root layout). Renders nothing.
 */
export function ViewController() {
  useEffect(() => {
    initZoom();

    const onKeyDown = (e: KeyboardEvent) => {
      // Accept both Ctrl (Win/Linux) and Cmd (macOS).
      if (!e.ctrlKey && !e.metaKey) return;

      // `key` covers the numpad and shifted glyphs across keyboard layouts.
      let handled = true;
      switch (e.key) {
        case "+":
        case "=": // unshifted "+" key on US layouts
          zoomIn();
          break;
        case "-":
        case "_":
          zoomOut();
          break;
        case "0":
          zoomReset();
          break;
        default:
          handled = false;
      }
      if (handled) {
        // Consume it here so the key never reaches the terminal / the PTY.
        e.preventDefault();
        e.stopPropagation();
      }
    };

    // Capture phase so the terminal's own key handler can't swallow it first.
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () =>
      window.removeEventListener("keydown", onKeyDown, { capture: true });
  }, []);

  return null;
}
