"use client";

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";

export default function DiagnosticsPage() {
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function handleCheck() {
    setRunning(true);
    setResult(null);
    setError(null);
    try {
      const res = await invoke<string>("cmd_check_environment");
      setResult(res);
    } catch (err) {
      setError(String(err));
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Diagnostics</h1>
        <p className="text-muted-foreground mt-1">
          Verify active development tools and environment health
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Environment Check</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <Button onClick={handleCheck} disabled={running}>
            {running ? "Running..." : "Run Diagnostics"}
          </Button>

          {result && (
            <div className="p-3 rounded-md bg-green-50 dark:bg-green-950 text-sm text-green-700 dark:text-green-300 whitespace-pre-wrap font-mono">
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
          <CardTitle className="text-sm font-medium">What is checked</CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="list-disc list-inside space-y-1 text-sm text-muted-foreground">
            <li>Rust toolchain: cargo, rustc, rustup</li>
            <li>Python: python, pip, uv, pipx</li>
            <li>Node/JS: node, npm, pnpm, yarn, bun, deno</li>
            <li>Go: go</li>
            <li>Systems: git, docker, docker-compose, curl, make</li>
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
