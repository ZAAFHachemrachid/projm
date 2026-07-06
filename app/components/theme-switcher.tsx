"use client";

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Check, Palette, Paintbrush, RotateCcw } from "lucide-react";
import { useTheme } from "@/hooks/use-theme";
import { ThemeTokens, fullTokens } from "@/lib/themes";

// Tokens exposed as color pickers in the custom editor — the highest-impact
// surfaces. Everything else inherits from the chosen base theme.
const CUSTOM_FIELDS: { key: string; label: string }[] = [
  { key: "background", label: "Background" },
  { key: "foreground", label: "Text" },
  { key: "card", label: "Card / Panel" },
  { key: "primary", label: "Primary" },
  { key: "accent", label: "Accent" },
  { key: "muted-foreground", label: "Muted text" },
  { key: "border", label: "Border" },
  { key: "sidebar", label: "Sidebar" },
];

// A compact palette preview: background block with primary/accent/foreground dots.
function Swatch({ tokens }: { tokens: ThemeTokens }) {
  return (
    <div
      className="flex items-center gap-1.5 h-11 rounded-md px-2.5 border border-border overflow-hidden"
      style={{ background: tokens.background }}
    >
      <span
        className="text-[11px] font-semibold"
        style={{ color: tokens.foreground }}
      >
        Aa
      </span>
      <span className="ml-auto size-4 rounded-full" style={{ background: tokens.primary }} />
      <span className="size-4 rounded-full" style={{ background: tokens.accent }} />
      <span
        className="size-4 rounded-full"
        style={{ background: tokens.card, border: `1px solid ${tokens.border}` }}
      />
    </div>
  );
}

export function ThemeSwitcher() {
  const { themeId, setThemeId, customTokens, setCustomTokens, themes } = useTheme();

  // Seed the custom editor from the current custom tokens, or from Projm Dark
  // if the user has never built one yet.
  const seed =
    customTokens && Object.keys(customTokens).length > 0
      ? customTokens
      : fullTokens("default");
  const [draft, setDraft] = useState<ThemeTokens>(seed);

  const isCustom = themeId === "custom";

  function startFromBase(baseId: string) {
    const base = { ...fullTokens(baseId) };
    setDraft(base);
    setCustomTokens(base);
    setThemeId("custom");
  }

  function updateField(key: string, value: string) {
    const next = { ...draft, [key]: value };
    setDraft(next);
    setCustomTokens(next);
  }

  return (
    <div className="space-y-6 animate-in fade-in duration-200">
      {/* Built-in theme grid */}
      <Card className="border border-border bg-card/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden transition-all duration-300 hover:border-border">
        <CardHeader className="p-6 pb-4">
          <CardTitle className="text-xs font-bold tracking-widest uppercase text-primary flex items-center gap-2.5">
            <Palette className="size-4.5 text-primary" />
            Theme
          </CardTitle>
        </CardHeader>
        <CardContent className="p-6 pt-0 space-y-5">
          <p className="text-sm text-muted-foreground leading-relaxed">
            Pick a look for the app. Themes apply instantly and persist across
            restarts. Bring your own below.
          </p>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3.5">
            {themes.map((theme) => {
              const active = themeId === theme.id;
              return (
                <button
                  key={theme.id}
                  type="button"
                  onClick={() => setThemeId(theme.id)}
                  className={`text-left rounded-xl border p-4 space-y-3 transition-all duration-200 ${
                    active
                      ? "border-primary/50 bg-primary/5 shadow-[0_0_16px_rgba(99,102,241,0.10)]"
                      : "border-border bg-background/20 hover:border-border hover:bg-accent"
                  }`}
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-sm font-semibold text-foreground">
                      {theme.name}
                    </span>
                    {active && (
                      <span className="inline-flex items-center gap-1 text-[10px] font-bold uppercase tracking-wide text-primary">
                        <Check className="size-3" /> Active
                      </span>
                    )}
                  </div>
                  <Swatch tokens={fullTokens(theme.id)} />
                  <p className="text-[11px] text-muted-foreground leading-relaxed">
                    {theme.description}
                  </p>
                </button>
              );
            })}

            {/* Custom slot */}
            <button
              type="button"
              onClick={() => setThemeId("custom")}
              className={`text-left rounded-xl border p-4 space-y-3 transition-all duration-200 ${
                isCustom
                  ? "border-fuchsia-500/50 bg-fuchsia-500/5 shadow-[0_0_16px_rgba(217,70,239,0.10)]"
                  : "border-dashed border-border bg-background/20 hover:border-border hover:bg-accent"
              }`}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="text-sm font-semibold text-foreground flex items-center gap-2">
                  <Paintbrush className="size-3.5 text-fuchsia-400" />
                  Custom
                </span>
                {isCustom && (
                  <span className="inline-flex items-center gap-1 text-[10px] font-bold uppercase tracking-wide text-fuchsia-300">
                    <Check className="size-3" /> Active
                  </span>
                )}
              </div>
              <Swatch tokens={draft} />
              <p className="text-[11px] text-muted-foreground leading-relaxed">
                Your own palette — edit it below.
              </p>
            </button>
          </div>
        </CardContent>
      </Card>

      {/* Custom theme editor */}
      <Card className="border border-border bg-card/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden transition-all duration-300 hover:border-border">
        <CardHeader className="p-6 pb-4 border-b border-border bg-card/20">
          <CardTitle className="text-xs font-bold tracking-widest uppercase text-fuchsia-400 flex items-center gap-2.5">
            <Paintbrush className="size-4.5 text-fuchsia-400" />
            Bring Your Own
          </CardTitle>
        </CardHeader>
        <CardContent className="p-6 space-y-5">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-xs text-muted-foreground">Start from:</span>
            {themes.map((theme) => (
              <button
                key={theme.id}
                type="button"
                onClick={() => startFromBase(theme.id)}
                className="text-[11px] font-semibold px-2.5 py-1 rounded-md border border-border bg-muted/60 text-foreground hover:text-foreground hover:border-border transition-colors"
              >
                {theme.name}
              </button>
            ))}
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {CUSTOM_FIELDS.map((field) => {
              const value = draft[field.key] || "#000000";
              return (
                <label
                  key={field.key}
                  className="flex items-center gap-3 rounded-lg border border-border bg-background/30 px-3 py-2"
                >
                  <input
                    type="color"
                    value={/^#[0-9a-fA-F]{6}$/.test(value) ? value : "#000000"}
                    onChange={(e) => updateField(field.key, e.target.value)}
                    className="size-7 shrink-0 cursor-pointer rounded border border-border bg-transparent"
                    aria-label={field.label}
                  />
                  <span className="text-xs text-foreground flex-1">{field.label}</span>
                  <input
                    type="text"
                    value={value}
                    onChange={(e) => updateField(field.key, e.target.value)}
                    className="w-24 font-mono text-[11px] text-muted-foreground bg-transparent outline-none text-right"
                    spellCheck={false}
                  />
                </label>
              );
            })}
          </div>

          <div className="flex items-center gap-3">
            <Button
              onClick={() => setThemeId("custom")}
              className="bg-fuchsia-600 hover:bg-fuchsia-500 text-foreground font-semibold h-9 px-4 shadow-sm transition-all duration-150 active:scale-95"
            >
              <Check className="size-3.5 mr-1.5" />
              Use custom theme
            </Button>
            <button
              type="button"
              onClick={() => {
                const base = { ...fullTokens("default") };
                setDraft(base);
                setCustomTokens(base);
              }}
              className="inline-flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
            >
              <RotateCcw className="size-3.5" />
              Reset to Projm Dark
            </button>
          </div>

          <p className="text-[11px] text-muted-foreground leading-relaxed">
            Custom themes are stored locally in this app. Tokens not shown here
            inherit from the base theme you started from.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
