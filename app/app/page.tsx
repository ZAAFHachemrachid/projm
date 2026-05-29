"use client";

import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { invoke } from "@tauri-apps/api/core";

interface Config {
  base: string;
}

const CATEGORIES = [
  { name: "apps", color: "bg-blue-500" },
  { name: "services", color: "bg-cyan-500" },
  { name: "ui", color: "bg-purple-500" },
  { name: "embedded", color: "bg-yellow-500" },
  { name: "ml", color: "bg-green-500" },
  { name: "tools", color: "bg-orange-500" },
  { name: "labs", color: "bg-gray-500" },
  { name: "content", color: "bg-pink-500" },
];

export default function DashboardPage() {
  const [config, setConfig] = useState<Config | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
      try {
        const cfg = await invoke<Config>("cmd_get_config");
        setConfig(cfg);
      } catch (err) {
        console.error("Failed to load config", err);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Dashboard</h1>
        <p className="text-muted-foreground mt-1">
          Overview of your project organization
        </p>
      </div>

      {/* Config info */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Base Directory</CardTitle>
        </CardHeader>
        <CardContent>
          {loading ? (
            <p className="text-sm text-muted-foreground">Loading...</p>
          ) : (
            <p className="text-sm font-mono">{config?.base ?? "Not configured"}</p>
          )}
        </CardContent>
      </Card>

      {/* Category overview */}
      <Card>
        <CardHeader>
          <CardTitle>Categories</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            {CATEGORIES.map((cat) => (
              <div
                key={cat.name}
                className="flex items-center gap-2 p-3 rounded-lg border"
              >
                <div className={`w-3 h-3 rounded-full ${cat.color}`} />
                <span className="text-sm font-medium capitalize">{cat.name}</span>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* Quick actions */}
      <Card>
        <CardHeader>
          <CardTitle>Quick Actions</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-3">
          <a
            href="/scan"
            className="inline-flex items-center gap-2 px-4 py-2 rounded-md bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 transition-colors"
          >
            Scan a Directory
          </a>
          <a
            href="/diagnostics"
            className="inline-flex items-center gap-2 px-4 py-2 rounded-md bg-secondary text-secondary-foreground text-sm font-medium hover:bg-secondary/80 transition-colors"
          >
            Run Diagnostics
          </a>
          <a
            href="/projects"
            className="inline-flex items-center gap-2 px-4 py-2 rounded-md bg-secondary text-secondary-foreground text-sm font-medium hover:bg-secondary/80 transition-colors"
          >
            Browse Projects
          </a>
        </CardContent>
      </Card>
    </div>
  );
}
