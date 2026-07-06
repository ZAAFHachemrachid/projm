// Theme system — a "theme" is nothing more than a map of semantic CSS-variable
// tokens applied to <html>. Built-in themes and user "bring-your-own" custom
// themes share the exact same shape, so the switcher, the setter, and the
// custom editor all operate on one primitive: ThemeTokens.

export type ThemeTokens = Record<string, string>;

export interface Theme {
  id: string;
  name: string;
  description: string;
  author?: string;
  /** Empty for "default" — clearing inline props reverts to globals.css. */
  tokens: ThemeTokens;
}

// Every token the setter manages. Applying a theme sets present keys and
// removes absent ones, so switching never leaves stale overrides behind.
export const THEME_TOKEN_KEYS = [
  "background",
  "foreground",
  "card",
  "card-foreground",
  "popover",
  "popover-foreground",
  "primary",
  "primary-foreground",
  "secondary",
  "secondary-foreground",
  "muted",
  "muted-foreground",
  "accent",
  "accent-foreground",
  "destructive",
  "border",
  "input",
  "ring",
  "sidebar",
  "sidebar-foreground",
  "sidebar-primary",
  "sidebar-primary-foreground",
  "sidebar-accent",
  "sidebar-accent-foreground",
  "sidebar-border",
  "sidebar-ring",
] as const;

export const THEME_STORAGE_KEY = "projm-theme";
export const CUSTOM_STORAGE_KEY = "projm-custom-theme";

// Concrete hex approximation of the built-in projm dark palette. The real
// "default" theme applies EMPTY tokens (exact globals.css look, zero drift);
// this seed exists only to paint swatches and to seed the custom editor.
export const DEFAULT_SEED: ThemeTokens = {
  background: "#0e1116",
  foreground: "#f0f2f4",
  card: "#14171d",
  "card-foreground": "#f0f2f4",
  popover: "#12151b",
  "popover-foreground": "#f0f2f4",
  primary: "#4f46e5",
  "primary-foreground": "#ffffff",
  secondary: "#1d2027",
  "secondary-foreground": "#f0f2f4",
  muted: "#191c22",
  "muted-foreground": "#7e8590",
  accent: "#1b1e25",
  "accent-foreground": "#fafbfc",
  destructive: "#e5484d",
  border: "#23262e",
  input: "#23262e",
  ring: "#4f46e5",
  sidebar: "#101319",
  "sidebar-foreground": "#e6e9ec",
  "sidebar-primary": "#d3d6db",
  "sidebar-primary-foreground": "#0e1116",
  "sidebar-accent": "#171a20",
  "sidebar-accent-foreground": "#fafbfc",
  "sidebar-border": "#23262e",
  "sidebar-ring": "#6b7280",
};

const nightOwl: ThemeTokens = {
  background: "#011627",
  foreground: "#d6deeb",
  card: "#0b2942",
  "card-foreground": "#d6deeb",
  popover: "#0b2942",
  "popover-foreground": "#d6deeb",
  primary: "#82aaff",
  "primary-foreground": "#011627",
  secondary: "#1d3b53",
  "secondary-foreground": "#d6deeb",
  muted: "#1d3b53",
  "muted-foreground": "#637777",
  accent: "#13344a",
  "accent-foreground": "#ecf0f1",
  destructive: "#ef5350",
  border: "#122d42",
  input: "#122d42",
  ring: "#82aaff",
  sidebar: "#010e1a",
  "sidebar-foreground": "#d6deeb",
  "sidebar-primary": "#82aaff",
  "sidebar-primary-foreground": "#011627",
  "sidebar-accent": "#0b2942",
  "sidebar-accent-foreground": "#ecf0f1",
  "sidebar-border": "#122d42",
  "sidebar-ring": "#82aaff",
};

const catppuccinMocha: ThemeTokens = {
  background: "#1e1e2e",
  foreground: "#cdd6f4",
  card: "#181825",
  "card-foreground": "#cdd6f4",
  popover: "#11111b",
  "popover-foreground": "#cdd6f4",
  primary: "#cba6f7",
  "primary-foreground": "#1e1e2e",
  secondary: "#313244",
  "secondary-foreground": "#cdd6f4",
  muted: "#313244",
  "muted-foreground": "#a6adc8",
  accent: "#45475a",
  "accent-foreground": "#cdd6f4",
  destructive: "#f38ba8",
  border: "#45475a",
  input: "#45475a",
  ring: "#cba6f7",
  sidebar: "#181825",
  "sidebar-foreground": "#cdd6f4",
  "sidebar-primary": "#cba6f7",
  "sidebar-primary-foreground": "#1e1e2e",
  "sidebar-accent": "#313244",
  "sidebar-accent-foreground": "#cdd6f4",
  "sidebar-border": "#313244",
  "sidebar-ring": "#cba6f7",
};

const githubDark: ThemeTokens = {
  background: "#0d1117",
  foreground: "#e6edf3",
  card: "#161b22",
  "card-foreground": "#e6edf3",
  popover: "#161b22",
  "popover-foreground": "#e6edf3",
  primary: "#2f81f7",
  "primary-foreground": "#ffffff",
  secondary: "#21262d",
  "secondary-foreground": "#e6edf3",
  muted: "#21262d",
  "muted-foreground": "#8b949e",
  accent: "#21262d",
  "accent-foreground": "#e6edf3",
  destructive: "#f85149",
  border: "#30363d",
  input: "#30363d",
  ring: "#2f81f7",
  sidebar: "#0d1117",
  "sidebar-foreground": "#e6edf3",
  "sidebar-primary": "#2f81f7",
  "sidebar-primary-foreground": "#ffffff",
  "sidebar-accent": "#161b22",
  "sidebar-accent-foreground": "#e6edf3",
  "sidebar-border": "#30363d",
  "sidebar-ring": "#2f81f7",
};

// Ordered list — this drives the switcher grid. Add a new built-in here.
export const BUILTIN_THEMES: Theme[] = [
  {
    id: "default",
    name: "Projm Dark",
    description: "The built-in slate-blue dark theme.",
    tokens: {},
  },
  {
    id: "night-owl",
    name: "Night Owl",
    description: "Sarah Drasner's deep-navy palette for night owls.",
    author: "Sarah Drasner",
    tokens: nightOwl,
  },
  {
    id: "catppuccin-mocha",
    name: "Catppuccin Mocha",
    description: "Soothing pastel mauve on warm dark.",
    author: "Catppuccin",
    tokens: catppuccinMocha,
  },
  {
    id: "github-dark",
    name: "GitHub Dark",
    description: "GitHub's default dark canvas.",
    author: "GitHub",
    tokens: githubDark,
  },
];

const BUILTIN_BY_ID: Record<string, Theme> = Object.fromEntries(
  BUILTIN_THEMES.map((t) => [t.id, t]),
);

/** Concrete, fully-populated token map for a built-in id (default → seed). */
export function fullTokens(id: string): ThemeTokens {
  if (id === "default") return DEFAULT_SEED;
  return BUILTIN_BY_ID[id]?.tokens ?? DEFAULT_SEED;
}

/** The tokens the setter should apply for a given selection. */
export function resolveTokens(id: string, customTokens: ThemeTokens): ThemeTokens {
  if (id === "custom") return customTokens;
  return BUILTIN_BY_ID[id]?.tokens ?? {};
}

/** Write tokens as inline custom properties; remove any not present. */
export function applyTheme(tokens: ThemeTokens): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  for (const key of THEME_TOKEN_KEYS) {
    const value = tokens[key];
    if (value) root.style.setProperty(`--${key}`, value);
    else root.style.removeProperty(`--${key}`);
  }
}

export function loadCustomTokens(): ThemeTokens {
  if (typeof localStorage === "undefined") return {};
  try {
    return JSON.parse(localStorage.getItem(CUSTOM_STORAGE_KEY) || "{}") || {};
  } catch {
    return {};
  }
}

// Self-executing script embedded in <head> so the saved theme is applied
// before first paint — no flash of the default palette on launch.
export function themeInitScript(): string {
  const keys = JSON.stringify(THEME_TOKEN_KEYS);
  const themes = JSON.stringify(
    Object.fromEntries(BUILTIN_THEMES.map((t) => [t.id, t.tokens])),
  );
  return `(function(){try{var K=${keys},T=${themes};var id=localStorage.getItem('${THEME_STORAGE_KEY}')||'default';var tok={};if(id==='custom'){try{tok=JSON.parse(localStorage.getItem('${CUSTOM_STORAGE_KEY}')||'{}')||{};}catch(e){}}else if(T[id]){tok=T[id];}var r=document.documentElement;for(var i=0;i<K.length;i++){var k=K[i];if(tok[k]){r.style.setProperty('--'+k,tok[k]);}else{r.style.removeProperty('--'+k);}}}catch(e){}})();`;
}
