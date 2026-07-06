"use client";

import { useState } from "react";
import { ArrowLeft } from "lucide-react";
import { useHotkey } from "@tanstack/react-hotkeys";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { invoke } from "@tauri-apps/api/core";

export function ScanPanel({ onClose }: { onClose: () => void }) {
  const [path, setPath] = useState("");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function handleScan() {
    if (!path.trim()) return;
    setRunning(true);
    setResult(null);
    setError(null);
    try {
      const res = await invoke<string>("cmd_scan_directory", {
        path: path.trim(),
        dryRun: false,
      });
      setResult(res);
    } catch (err) {
      setError(String(err));
    } finally {
      setRunning(false);
    }
  }

  // Escape closes the scan overlay.
  useHotkey("Escape", () => {
    onClose();
  });

  return (
    <div className="space-y-6">
      <div className="flex items-start gap-3">
        <button
          type="button"
          onClick={onClose}
          className="group inline-flex items-center justify-center p-1.5 mt-1 rounded-lg border border-border bg-muted/60 text-muted-foreground hover:text-foreground hover:border-border hover:bg-accent transition-all duration-200"
          title="Close scan"
        >
          <ArrowLeft className="size-5 transition-transform group-hover:-translate-x-0.5" />
        </button>
        <div>
          <h1 className="text-2xl font-bold">Scan &amp; Organize</h1>
          <p className="text-muted-foreground mt-1">
            Scan a directory of projects and organize them into your base directory
          </p>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Directory to scan</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-2">
            <Input
              placeholder="/path/to/projects"
              value={path}
              onChange={(e) => setPath(e.target.value)}
            />
            <Button onClick={handleScan} disabled={running || !path.trim()}>
              {running ? "Scanning..." : "Scan"}
            </Button>
          </div>

          {result && (
            <div className="p-3 rounded-md bg-green-50 dark:bg-green-950 text-sm text-green-700 dark:text-green-300">
              {result}
            </div>
          )}

          {error && (
            <div className="p-3 rounded-md bg-red-50 dark:bg-red-950 text-sm text-red-700 dark:text-red-300">
              {error}
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">How it works</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2 text-sm text-muted-foreground">
          <p>1. Enter the path to a directory containing your projects.</p>
          <p>
            2. Projm scans each subdirectory, detects the stack (Cargo.toml,
            package.json, etc.), and classifies it into a category.
          </p>
          <p>
            3. Projects with shared prefixes and known suffixes
            (e.g. drivetrack-api + drivetrack-web) are grouped together.
          </p>
          <p>4. All projects are moved into your base directory.</p>
        </CardContent>
      </Card>
    </div>
  );
}
