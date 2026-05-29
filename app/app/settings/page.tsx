"use client";

import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { invoke } from "@tauri-apps/api/core";

interface Config {
  base: string;
}

export default function SettingsPage() {
  const [config, setConfig] = useState<Config | null>(null);
  const [loading, setLoading] = useState(true);
  const [baseDir, setBaseDir] = useState("");

  useEffect(() => {
    async function load() {
      try {
        const cfg = await invoke<Config>("cmd_get_config");
        setConfig(cfg);
        setBaseDir(cfg.base);
      } catch {
        // Rely on fallback below
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Settings</h1>
        <p className="text-muted-foreground mt-1">
          Configure projm preferences
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Base Directory</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {loading ? (
            <p className="text-sm text-muted-foreground">Loading...</p>
          ) : (
            <>
              <p className="text-sm text-muted-foreground">
                All organized projects are stored under this directory.
              </p>
              <div className="flex gap-2">
                <Input
                  value={baseDir}
                  onChange={(e) => setBaseDir(e.target.value)}
                  className="font-mono text-sm"
                />
                <Button variant="outline" disabled>
                  Save
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                Currently set to: {config?.base ?? "~/"}&#8203;projects
              </p>
            </>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Editors</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            Detected editors will appear here. Use the CLI{" "}
            <code className="text-xs bg-muted px-1 py-0.5 rounded">projm editors</code>{" "}
            command to list them.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
