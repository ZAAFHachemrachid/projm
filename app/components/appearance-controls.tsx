"use client";

import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ZoomIn, Type, RotateCcw } from "lucide-react";
import {
  getZoom,
  setZoom,
  zoomReset,
  ZOOM_EVENT,
  ZOOM_MIN,
  ZOOM_MAX,
  ZOOM_STEP,
  ZOOM_DEFAULT,
  getTermFontSize,
  setTermFontSize,
  TERM_FONT_MIN,
  TERM_FONT_MAX,
  TERM_FONT_DEFAULT,
} from "@/lib/view-prefs";

const pct = (z: number) => `${Math.round(z * 100)}%`;

/** Whole-app zoom control (mirrors the Ctrl +/- shortcuts). */
export function ZoomControl() {
  const [zoom, setZoomState] = useState(ZOOM_DEFAULT);

  // Initialise from storage and stay in sync when the keyboard shortcuts fire.
  useEffect(() => {
    setZoomState(getZoom());
    const onZoom = (e: Event) => {
      const z = (e as CustomEvent<number>).detail;
      if (typeof z === "number") setZoomState(z);
    };
    window.addEventListener(ZOOM_EVENT, onZoom);
    return () => window.removeEventListener(ZOOM_EVENT, onZoom);
  }, []);

  return (
    <Card className="border border-border bg-card/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden transition-all duration-300 hover:border-border">
      <CardHeader className="p-6 pb-4">
        <CardTitle className="text-xs font-bold tracking-widest uppercase text-primary flex items-center gap-2.5">
          <ZoomIn className="size-4.5 text-primary" />
          Interface Zoom
        </CardTitle>
      </CardHeader>
      <CardContent className="p-6 pt-0 space-y-5">
        <p className="text-sm text-muted-foreground leading-relaxed">
          Scales the entire interface — sidebar, panels, and terminal together.
          Also bound to{" "}
          <kbd className="text-[10px] bg-muted/60 text-primary px-1.5 py-0.5 rounded border border-border font-mono">
            Ctrl&nbsp;+
          </kbd>{" "}
          <kbd className="text-[10px] bg-muted/60 text-primary px-1.5 py-0.5 rounded border border-border font-mono">
            Ctrl&nbsp;−
          </kbd>{" "}
          and{" "}
          <kbd className="text-[10px] bg-muted/60 text-primary px-1.5 py-0.5 rounded border border-border font-mono">
            Ctrl&nbsp;0
          </kbd>{" "}
          to reset.
        </p>
        <div className="flex items-center gap-4">
          <input
            type="range"
            min={ZOOM_MIN}
            max={ZOOM_MAX}
            step={ZOOM_STEP}
            value={zoom}
            onChange={(e) => setZoomState(setZoom(Number(e.target.value)))}
            className="flex-1 accent-primary cursor-pointer"
            aria-label="Interface zoom"
          />
          <span className="w-14 text-right font-mono text-sm text-foreground tabular-nums">
            {pct(zoom)}
          </span>
          <Button
            variant="secondary"
            onClick={() => setZoomState(zoomReset())}
            disabled={zoom === ZOOM_DEFAULT}
            className="h-9 px-3 text-xs font-semibold disabled:opacity-40"
            title="Reset to 100%"
          >
            <RotateCcw className="size-3.5 mr-1.5" />
            Reset
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

/** Independent terminal font-size control (does not affect the rest of the UI). */
export function TerminalFontControl() {
  const [size, setSizeState] = useState(TERM_FONT_DEFAULT);

  useEffect(() => {
    setSizeState(getTermFontSize());
  }, []);

  return (
    <Card className="border border-border bg-card/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden transition-all duration-300 hover:border-border">
      <CardHeader className="p-6 pb-4 border-b border-border bg-card/20">
        <CardTitle className="text-xs font-bold tracking-widest uppercase text-cyan-400 flex items-center gap-2.5">
          <Type className="size-4.5 text-cyan-400" />
          Terminal Font Size
        </CardTitle>
      </CardHeader>
      <CardContent className="p-6 space-y-5">
        <p className="text-sm text-muted-foreground leading-relaxed">
          Font size for the embedded terminal only. Applies immediately to open
          terminals and every new tab.
        </p>
        <div className="flex items-center gap-4">
          <input
            type="range"
            min={TERM_FONT_MIN}
            max={TERM_FONT_MAX}
            step={1}
            value={size}
            onChange={(e) => setSizeState(setTermFontSize(Number(e.target.value)))}
            className="flex-1 accent-cyan-500 cursor-pointer"
            aria-label="Terminal font size"
          />
          <span className="w-14 text-right font-mono text-sm text-foreground tabular-nums">
            {size}px
          </span>
          <Button
            variant="secondary"
            onClick={() => setSizeState(setTermFontSize(TERM_FONT_DEFAULT))}
            disabled={size === TERM_FONT_DEFAULT}
            className="h-9 px-3 text-xs font-semibold disabled:opacity-40"
            title={`Reset to ${TERM_FONT_DEFAULT}px`}
          >
            <RotateCcw className="size-3.5 mr-1.5" />
            Reset
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
