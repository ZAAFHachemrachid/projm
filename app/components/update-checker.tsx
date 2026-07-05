"use client";

import { useEffect, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type UpdateState = "idle" | "available" | "downloading" | "ready" | "error";

export function UpdateChecker() {
  const [state, setState] = useState<UpdateState>("idle");
  const [update, setUpdate] = useState<Update | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    check()
      .then((result) => {
        if (!cancelled && result) {
          setUpdate(result);
          setState("available");
        }
      })
      .catch(() => {
        // Offline or updater endpoint unreachable — stay silent.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  async function installUpdate() {
    if (!update) return;
    setState("downloading");
    try {
      await update.downloadAndInstall();
      setState("ready");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setState("error");
    }
  }

  if (state === "idle" || !update) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 w-80 rounded-lg border border-border bg-card p-4 shadow-lg">
      {state === "available" && (
        <>
          <p className="text-sm font-medium">Update available</p>
          <p className="mt-1 text-xs text-muted-foreground">
            Projm v{update.version} is ready to download.
          </p>
          <div className="mt-3 flex gap-2">
            <button
              onClick={installUpdate}
              className="rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:opacity-90"
            >
              Download & install
            </button>
            <button
              onClick={() => setState("idle")}
              className="rounded-md px-3 py-1.5 text-xs text-muted-foreground hover:text-foreground"
            >
              Later
            </button>
          </div>
        </>
      )}
      {state === "downloading" && (
        <p className="text-sm text-muted-foreground">
          Downloading Projm v{update.version}…
        </p>
      )}
      {state === "ready" && (
        <>
          <p className="text-sm font-medium">Update installed</p>
          <p className="mt-1 text-xs text-muted-foreground">
            Restart Projm to finish updating.
          </p>
          <div className="mt-3 flex gap-2">
            <button
              onClick={() => relaunch()}
              className="rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:opacity-90"
            >
              Restart now
            </button>
            <button
              onClick={() => setState("idle")}
              className="rounded-md px-3 py-1.5 text-xs text-muted-foreground hover:text-foreground"
            >
              Later
            </button>
          </div>
        </>
      )}
      {state === "error" && (
        <p className="text-xs text-destructive">Update failed: {error}</p>
      )}
    </div>
  );
}
