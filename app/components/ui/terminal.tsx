"use client";

import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface TerminalViewProps {
  cwd: string;
}

export default function TerminalView({ cwd }: TerminalViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<any>(null);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;
    let cleanupResize: (() => void) | null = null;

    async function initTerminal() {
      // Dynamically import xterm to prevent SSR build failures
      const { Terminal } = await import("@xterm/xterm");
      const { FitAddon } = await import("@xterm/addon-fit");
      await import("@xterm/xterm/css/xterm.css");

      if (!active || !containerRef.current) return;

      // Initialize xterm
      const term = new Terminal({
        cursorBlink: true,
        fontFamily: "Geist Mono, Menlo, Monaco, Courier New, monospace",
        fontSize: 13,
        lineHeight: 1.2,
        theme: {
          background: "#0c0d0e", // Carbon background
          foreground: "#e2e8f0",
          cursor: "#cbd5e1",
          black: "#1e293b",
          red: "#ef4444",
          green: "#22c55e",
          yellow: "#eab308",
          blue: "#3b82f6",
          magenta: "#a855f7",
          cyan: "#06b6d4",
          white: "#f8fafc",
        },
      });

      const fitAddon = new FitAddon();
      term.loadAddon(fitAddon);
      term.open(containerRef.current);
      fitAddon.fit();

      termRef.current = term;

      // Listen to keystrokes and pipe to this project's PTY session
      term.onData((data) => {
        invoke("cmd_write_terminal", { cwd, input: data }).catch((err) =>
          console.error("Failed to write to terminal stdin", err)
        );
      });

      // Stream stdout/stderr from Rust; sessions are keyed by cwd, so only
      // render output belonging to this terminal's project.
      const unsubscribe = await listen<{ cwd: string; data: string }>(
        "terminal-data",
        (event) => {
          if (active && event.payload.cwd === cwd) {
            term.write(event.payload.data);
          }
        }
      );
      unlisten = unsubscribe;

      // Spawn backend process
      try {
        await invoke("cmd_spawn_terminal", { cwd });
      } catch (err) {
        term.write(`\r\n\x1b[31mFailed to spawn shell process in ${cwd}: ${err}\x1b[0m\r\n`);
      }

      // Handle resize
      const handleResize = () => {
        if (fitAddon) {
          fitAddon.fit();
        }
      };
      window.addEventListener("resize", handleResize);
      cleanupResize = () => window.removeEventListener("resize", handleResize);

      // Fit again after short delay to let layout settle
      setTimeout(() => {
        if (active && fitAddon) {
          fitAddon.fit();
        }
      }, 200);
    }

    initTerminal();

    return () => {
      active = false;
      if (unlisten) unlisten();
      if (cleanupResize) cleanupResize();
      if (termRef.current) {
        termRef.current.dispose();
      }
    };
  }, [cwd]);

  return (
    <div className="w-full h-full bg-[#0c0d0e] p-3 rounded-lg border border-border overflow-hidden">
      <div ref={containerRef} className="w-full h-full" />
    </div>
  );
}
