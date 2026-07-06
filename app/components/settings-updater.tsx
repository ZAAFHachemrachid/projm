"use client";

import { useEffect, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import {
  DownloadCloud,
  RefreshCw,
  Check,
  AlertCircle,
  Sparkles,
  RotateCw,
} from "lucide-react";

type Status =
  | "idle"
  | "checking"
  | "uptodate"
  | "available"
  | "downloading"
  | "ready"
  | "error";

function formatBytes(n: number): string {
  if (!n) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(n) / Math.log(1024));
  return `${(n / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export function SettingsUpdater() {
  const [version, setVersion] = useState<string>("");
  const [status, setStatus] = useState<Status>("idle");
  const [update, setUpdate] = useState<Update | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<{ downloaded: number; total: number }>({
    downloaded: 0,
    total: 0,
  });

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(""));
  }, []);

  async function checkForUpdates() {
    setStatus("checking");
    setError(null);
    try {
      const result = await check();
      if (result) {
        setUpdate(result);
        setStatus("available");
      } else {
        setStatus("uptodate");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setStatus("error");
    }
  }

  async function installUpdate() {
    if (!update) return;
    setStatus("downloading");
    setProgress({ downloaded: 0, total: 0 });
    let downloaded = 0;
    let total = 0;
    try {
      await update.downloadAndInstall((e) => {
        switch (e.event) {
          case "Started":
            total = e.data.contentLength ?? 0;
            setProgress({ downloaded: 0, total });
            break;
          case "Progress":
            downloaded += e.data.chunkLength;
            setProgress({ downloaded, total });
            break;
          case "Finished":
            setProgress({ downloaded: total, total });
            break;
        }
      });
      setStatus("ready");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setStatus("error");
    }
  }

  const pct =
    progress.total > 0
      ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100))
      : 0;

  return (
    <div className="space-y-6 animate-in fade-in duration-200">
      <Card className="border border-border bg-card/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden transition-all duration-300 hover:border-foreground/10">
        <CardHeader className="p-6 pb-4">
          <CardTitle className="text-xs font-bold tracking-widest uppercase text-primary flex items-center gap-2.5">
            <DownloadCloud className="size-4.5 text-primary" />
            Software Updates
          </CardTitle>
        </CardHeader>
        <CardContent className="p-6 pt-0 space-y-5">
          <div className="flex items-center justify-between gap-4">
            <div className="space-y-1">
              <p className="text-sm text-muted-foreground leading-relaxed">
                Check for a newer release of Projm. Updates download and install
                in place — you restart once to finish.
              </p>
              <p className="text-xs text-muted-foreground">
                Current version:{" "}
                <code className="text-xs font-mono bg-muted/60 text-primary px-2 py-0.5 rounded border border-border">
                  v{version || "—"}
                </code>
              </p>
            </div>
            <Button
              onClick={checkForUpdates}
              disabled={status === "checking" || status === "downloading"}
              className="bg-primary hover:bg-primary/90 text-primary-foreground font-semibold h-10 px-5 shadow-sm transition-all duration-150 active:scale-95 disabled:opacity-50 disabled:pointer-events-none shrink-0"
            >
              {status === "checking" ? (
                <>
                  <RefreshCw className="size-3.5 mr-1.5 animate-spin" />
                  Checking…
                </>
              ) : (
                <>
                  <RefreshCw className="size-3.5 mr-1.5" />
                  Check for updates
                </>
              )}
            </Button>
          </div>

          {/* Status surface */}
          {status === "uptodate" && (
            <div className="flex items-center gap-2 rounded-lg border border-emerald-500/20 bg-emerald-500/5 px-4 py-3 text-sm text-emerald-400">
              <Check className="size-4 shrink-0" />
              You&rsquo;re on the latest version.
            </div>
          )}

          {status === "available" && update && (
            <div className="rounded-lg border border-primary/20 bg-primary/5 px-4 py-3.5 space-y-3">
              <div className="flex items-center gap-2 text-sm font-medium text-foreground">
                <Sparkles className="size-4 shrink-0 text-primary" />
                Projm v{update.version} is available.
              </div>
              {update.body && (
                <p className="text-xs text-muted-foreground leading-relaxed whitespace-pre-line max-h-32 overflow-y-auto scrollbar-thin scrollbar-thumb-muted scrollbar-track-transparent">
                  {update.body}
                </p>
              )}
              <Button
                onClick={installUpdate}
                className="bg-primary hover:bg-primary/90 text-primary-foreground font-semibold h-9 px-4 shadow-sm transition-all duration-150 active:scale-95"
              >
                <DownloadCloud className="size-3.5 mr-1.5" />
                Download &amp; install
              </Button>
            </div>
          )}

          {status === "downloading" && (
            <div className="rounded-lg border border-border bg-background/40 px-4 py-3.5 space-y-2.5">
              <div className="flex items-center justify-between text-xs text-muted-foreground">
                <span className="flex items-center gap-1.5">
                  <DownloadCloud className="size-3.5 text-primary" />
                  Downloading v{update?.version}…
                </span>
                <span className="font-mono">
                  {progress.total > 0
                    ? `${formatBytes(progress.downloaded)} / ${formatBytes(progress.total)} · ${pct}%`
                    : formatBytes(progress.downloaded)}
                </span>
              </div>
              <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
                <div
                  className="h-full rounded-full bg-primary transition-all duration-200"
                  style={{ width: progress.total > 0 ? `${pct}%` : "40%" }}
                />
              </div>
            </div>
          )}

          {status === "ready" && (
            <div className="rounded-lg border border-emerald-500/20 bg-emerald-500/5 px-4 py-3.5 space-y-3">
              <div className="flex items-center gap-2 text-sm font-medium text-emerald-400">
                <Check className="size-4 shrink-0" />
                Update installed. Restart to finish.
              </div>
              <Button
                onClick={() => relaunch()}
                className="bg-primary hover:bg-primary/90 text-primary-foreground font-semibold h-9 px-4 shadow-sm transition-all duration-150 active:scale-95"
              >
                <RotateCw className="size-3.5 mr-1.5" />
                Restart now
              </Button>
            </div>
          )}

          {status === "error" && (
            <div className="flex items-start gap-2 rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-xs text-destructive">
              <AlertCircle className="size-4 shrink-0 mt-0.5" />
              <span className="leading-relaxed break-all">
                Update failed: {error}
              </span>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
