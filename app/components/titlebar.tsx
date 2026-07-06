"use client";

import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { Search, Sparkles, X, Minus, Square } from "lucide-react";

export function Titlebar() {
  const [appWindow, setAppWindow] = useState<any>(null);

  useEffect(() => {
    // Dynamically fetch window instance to prevent SSR errors
    setAppWindow(getCurrentWindow());
  }, []);

  const handleSearchClick = () => {
    // Programmatically trigger search overlay modal
    const event = new KeyboardEvent("keydown", {
      key: "k",
      ctrlKey: true,
      bubbles: true,
    });
    window.dispatchEvent(event);
  };

  return (
    <div 
      data-tauri-drag-region 
      className="h-10 w-full flex items-center justify-between px-4 bg-background border-b border-border select-none shrink-0 relative z-50"
    >
      {/* Left: Branding Logo with Breathing Status Glow */}
      <div className="flex items-center gap-2.5" data-tauri-drag-region>
        <div className="relative flex items-center justify-center size-5 rounded bg-primary/10 border border-primary/20" data-tauri-drag-region>
          <Sparkles className="size-3 text-primary" />
          <span className="absolute -top-0.5 -right-0.5 size-1.5 rounded-full bg-emerald-400 shadow-[0_0_6px_#10b981]" />
        </div>
        <span className="text-[11px] font-bold tracking-wider text-foreground font-sans" data-tauri-drag-region>
          Projm
        </span>
        <span className="text-[9px] font-medium text-muted-foreground border border-border px-1.5 py-0.2 rounded bg-muted/30">
          v1.0.0
        </span>
      </div>

      {/* Center: Clean command launcher mapped directly to Ctrl+K */}
      <div 
        onClick={handleSearchClick}
        className="hidden sm:flex items-center justify-between px-3 py-1.5 bg-card/50 hover:bg-card border border-border rounded-lg w-72 text-muted-foreground text-[10px] cursor-pointer hover:border-border hover:text-muted-foreground transition-all duration-200"
      >
        <div className="flex items-center gap-1.5">
          <Search className="size-3 text-muted-foreground" />
          <span>Search or run commands...</span>
        </div>
        <span className="font-mono text-[9px] bg-muted border border-border px-1 rounded text-muted-foreground">Ctrl+K</span>
      </div>

      {/* Right: Modern minimalist window control buttons */}
      <div className="flex items-center gap-1">
        <button
          onClick={() => appWindow?.minimize()}
          className="size-6 rounded flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-accent active:scale-95 transition-all duration-150"
          aria-label="Minimize"
        >
          <Minus className="size-3.5" />
        </button>
        <button
          onClick={() => appWindow?.toggleMaximize()}
          className="size-6 rounded flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-accent active:scale-95 transition-all duration-150"
          aria-label="Maximize"
        >
          <Square className="size-2.5" />
        </button>
        <button
          onClick={() => appWindow?.close()}
          className="size-6 rounded flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-red-500/20 hover:text-red-300 active:scale-95 transition-all duration-150"
          aria-label="Close"
        >
          <X className="size-3.5" />
        </button>
      </div>
    </div>
  );
}
