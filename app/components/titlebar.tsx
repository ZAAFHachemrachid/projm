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
      className="h-10 w-full flex items-center justify-between px-4 bg-[#0d0e10] border-b border-white/5 select-none shrink-0 relative z-50"
    >
      {/* Left: Branding Logo with Breathing Status Glow */}
      <div className="flex items-center gap-2.5" data-tauri-drag-region>
        <div className="relative flex items-center justify-center size-5 rounded bg-indigo-500/10 border border-indigo-500/20" data-tauri-drag-region>
          <Sparkles className="size-3 text-indigo-400" />
          <span className="absolute -top-0.5 -right-0.5 size-1.5 rounded-full bg-emerald-400 shadow-[0_0_6px_#10b981]" />
        </div>
        <span className="text-[11px] font-bold tracking-wider text-slate-300 font-sans" data-tauri-drag-region>
          Projm
        </span>
        <span className="text-[9px] font-medium text-slate-500 border border-white/5 px-1.5 py-0.2 rounded bg-zinc-900/30">
          v1.0.0
        </span>
      </div>

      {/* Center: Clean command launcher mapped directly to Ctrl+K */}
      <div 
        onClick={handleSearchClick}
        className="hidden sm:flex items-center justify-between px-3 py-1.5 bg-[#17181c]/50 hover:bg-[#17181c] border border-white/5 rounded-lg w-72 text-zinc-500 text-[10px] cursor-pointer hover:border-white/10 hover:text-zinc-400 transition-all duration-200"
      >
        <div className="flex items-center gap-1.5">
          <Search className="size-3 text-zinc-600" />
          <span>Search or run commands...</span>
        </div>
        <span className="font-mono text-[9px] bg-zinc-900 border border-white/5 px-1 rounded text-zinc-600">Ctrl+K</span>
      </div>

      {/* Right: Modern minimalist window control buttons */}
      <div className="flex items-center gap-1">
        <button
          onClick={() => appWindow?.minimize()}
          className="size-6 rounded flex items-center justify-center text-zinc-400 hover:text-white hover:bg-white/5 active:scale-95 transition-all duration-150"
          aria-label="Minimize"
        >
          <Minus className="size-3.5" />
        </button>
        <button
          onClick={() => appWindow?.toggleMaximize()}
          className="size-6 rounded flex items-center justify-center text-zinc-400 hover:text-white hover:bg-white/5 active:scale-95 transition-all duration-150"
          aria-label="Maximize"
        >
          <Square className="size-2.5" />
        </button>
        <button
          onClick={() => appWindow?.close()}
          className="size-6 rounded flex items-center justify-center text-zinc-400 hover:text-white hover:bg-red-500/20 hover:text-red-300 active:scale-95 transition-all duration-150"
          aria-label="Close"
        >
          <X className="size-3.5" />
        </button>
      </div>
    </div>
  );
}
