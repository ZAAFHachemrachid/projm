// View preferences: whole-app zoom (Ctrl +/-) and terminal font size.
// Both persist to localStorage so they survive restarts, and both broadcast a
// window event so already-mounted components (open terminals, the settings
// sliders) live-update without a reload.

export const ZOOM_KEY = "projm.zoom";
export const TERM_FONT_KEY = "projm.termFontSize";

export const ZOOM_MIN = 0.5;
export const ZOOM_MAX = 2.0;
export const ZOOM_STEP = 0.1;
export const ZOOM_DEFAULT = 1.0;

export const TERM_FONT_MIN = 9;
export const TERM_FONT_MAX = 28;
export const TERM_FONT_DEFAULT = 13;

/** Fired (window event) whenever the app zoom factor changes. `detail` = factor. */
export const ZOOM_EVENT = "projm:zoom";
/** Fired (window event) whenever the terminal font size changes. `detail` = px. */
export const TERM_FONT_EVENT = "projm:term-font-size";

const clamp = (n: number, lo: number, hi: number) =>
  Math.min(hi, Math.max(lo, n));

// Round to one decimal so 0.1 steps don't accumulate float noise (0.30000004).
const round1 = (n: number) => Math.round(n * 10) / 10;

function readNumber(key: string, fallback: number): number {
  if (typeof window === "undefined") return fallback;
  const raw = window.localStorage.getItem(key);
  if (raw == null) return fallback;
  const n = Number.parseFloat(raw);
  return Number.isFinite(n) ? n : fallback;
}

// ── Whole-app zoom ────────────────────────────────────────────────────────────

export function getZoom(): number {
  return clamp(readNumber(ZOOM_KEY, ZOOM_DEFAULT), ZOOM_MIN, ZOOM_MAX);
}

/**
 * Apply the zoom to the actual webview. Uses Tauri's native webview zoom so the
 * whole surface (UI + terminal canvas) scales crisply, exactly like a browser's
 * Ctrl +/-. Falls back to CSS `zoom` when running outside Tauri (plain browser
 * dev) so the setting is still visible.
 */
async function applyZoom(factor: number): Promise<void> {
  try {
    const { getCurrentWebview } = await import("@tauri-apps/api/webview");
    await getCurrentWebview().setZoom(factor);
  } catch {
    // Not in Tauri, or the permission is missing — degrade to CSS zoom.
    if (typeof document !== "undefined") {
      (document.documentElement.style as unknown as { zoom: string }).zoom =
        String(factor);
    }
  }
}

/** Persist + apply a new zoom factor and notify listeners. Returns the clamped value. */
export function setZoom(factor: number): number {
  const value = round1(clamp(factor, ZOOM_MIN, ZOOM_MAX));
  if (typeof window !== "undefined") {
    window.localStorage.setItem(ZOOM_KEY, String(value));
    window.dispatchEvent(new CustomEvent(ZOOM_EVENT, { detail: value }));
  }
  void applyZoom(value);
  return value;
}

export const zoomIn = () => setZoom(getZoom() + ZOOM_STEP);
export const zoomOut = () => setZoom(getZoom() - ZOOM_STEP);
export const zoomReset = () => setZoom(ZOOM_DEFAULT);

/** Re-apply the persisted zoom to the webview (call once on startup). */
export function initZoom(): void {
  void applyZoom(getZoom());
}

// ── Terminal font size ────────────────────────────────────────────────────────

export function getTermFontSize(): number {
  return Math.round(
    clamp(readNumber(TERM_FONT_KEY, TERM_FONT_DEFAULT), TERM_FONT_MIN, TERM_FONT_MAX),
  );
}

/** Persist a new terminal font size and notify open terminals. Returns the clamped px. */
export function setTermFontSize(px: number): number {
  const value = Math.round(clamp(px, TERM_FONT_MIN, TERM_FONT_MAX));
  if (typeof window !== "undefined") {
    window.localStorage.setItem(TERM_FONT_KEY, String(value));
    window.dispatchEvent(new CustomEvent(TERM_FONT_EVENT, { detail: value }));
  }
  return value;
}

/**
 * The mono font actually registered by next/font (a hashed family name exposed
 * as the `--font-geist-mono` CSS variable), with web-safe fallbacks. Naming the
 * literal "Geist Mono" — as the terminal used to — never matches, forcing xterm
 * to measure against a fallback font and mis-size the grid.
 */
export function terminalFontFamily(): string {
  const fallbacks = 'Menlo, Monaco, "Courier New", monospace';
  if (typeof document === "undefined") return fallbacks;
  const loaded = getComputedStyle(document.documentElement)
    .getPropertyValue("--font-geist-mono")
    .trim();
  return loaded ? `${loaded}, ${fallbacks}` : fallbacks;
}
