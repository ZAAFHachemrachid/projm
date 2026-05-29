"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

const CATEGORIES = [
  { name: "apps", desc: "Full-stack, Tauri, Flutter, Android, iOS, monorepos" },
  { name: "services", desc: "Backend APIs — Rust, Hono, Go, Java, Express" },
  { name: "ui", desc: "Frontend-only — React, Svelte, Vue" },
  { name: "embedded", desc: "ESP32, LoRa, no_std Rust" },
  { name: "ml", desc: "ML pipelines, notebooks" },
  { name: "tools", desc: "CLI tools, scripts" },
  { name: "labs", desc: "Experiments and unclassified" },
  { name: "content", desc: "Documentation and content sites" },
];

export default function ProjectsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Projects</h1>
        <p className="text-muted-foreground mt-1">
          Browse projects organized by category
        </p>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {CATEGORIES.map((cat) => (
          <Card key={cat.name}>
            <CardHeader>
              <CardTitle className="text-base capitalize">{cat.name}</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground">{cat.desc}</p>
              <p className="text-xs text-muted-foreground mt-2 italic">
                Scan a directory to populate this category
              </p>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
