"use client";

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderGit2, FolderOpen, FolderPlus, Loader2, Sparkles } from "lucide-react";

interface Detected {
  category: string;
  reason: string;
}

interface AddedProject {
  path: string;
  category: string;
}

interface Props {
  categories: { id: string; name: string }[];
  base?: string;
  onClose: () => void;
  onAdded: (added: AddedProject) => void;
}

type Mode = "local" | "git";

export function AddProjectDialog({ categories, base, onClose, onAdded }: Props) {
  const [mode, setMode] = useState<Mode>("local");
  const [path, setPath] = useState("");
  const [url, setUrl] = useState("");
  const [name, setName] = useState("");
  const [branch, setBranch] = useState("");
  // "" = Auto: let the classifier pick the category.
  const [category, setCategory] = useState("");
  const [detected, setDetected] = useState<Detected | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function detect(p: string) {
    if (!p.trim()) {
      setDetected(null);
      return;
    }
    try {
      const res = await invoke<Detected>("cmd_rules_test", { path: p.trim() });
      setDetected(res);
    } catch {
      setDetected(null);
    }
  }

  async function browse() {
    const dir = await open({ directory: true, title: "Select project folder" });
    if (typeof dir === "string" && dir) {
      setPath(dir);
      setError(null);
      detect(dir);
    }
  }

  const canSubmit = mode === "local" ? path.trim().length > 0 : url.trim().length > 0;

  async function submit() {
    if (!canSubmit || busy) return;
    setBusy(true);
    setError(null);
    try {
      const added =
        mode === "local"
          ? await invoke<AddedProject>("cmd_add_project", {
              path: path.trim(),
              category: category || null,
            })
          : await invoke<AddedProject>("cmd_clone_project", {
              url: url.trim(),
              name: name.trim() || null,
              branch: branch.trim() || null,
              category: category || null,
            });
      onAdded(added);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  const autoLabel =
    mode === "local" && detected ? `auto · ${detected.category}` : "auto";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/60 backdrop-blur-sm">
      <div className="w-full max-w-md bg-card border border-border rounded-xl shadow-2xl overflow-hidden flex flex-col animate-in fade-in zoom-in-95 duration-150">
        {/* Header */}
        <div className="p-4 border-b border-border/20 flex items-center gap-2">
          <FolderPlus className="size-4 text-primary" />
          <span className="text-sm font-semibold text-foreground">Add project</span>
        </div>

        <div className="p-4 flex flex-col gap-4">
          {/* Source toggle */}
          <div className="grid grid-cols-2 gap-1.5">
            <button
              onClick={() => { setMode("local"); setError(null); }}
              className={`flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-md text-xs border transition-all ${
                mode === "local"
                  ? "bg-primary/15 text-primary border-primary/40"
                  : "bg-muted/20 text-muted-foreground border-border hover:text-foreground"
              }`}
            >
              <FolderOpen className="size-3.5" /> Local folder
            </button>
            <button
              onClick={() => { setMode("git"); setError(null); }}
              className={`flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-md text-xs border transition-all ${
                mode === "git"
                  ? "bg-primary/15 text-primary border-primary/40"
                  : "bg-muted/20 text-muted-foreground border-border hover:text-foreground"
              }`}
            >
              <FolderGit2 className="size-3.5" /> Git URL
            </button>
          </div>

          {mode === "local" ? (
            <div className="space-y-1.5">
              <label className="text-[9px] font-bold text-muted-foreground uppercase tracking-widest">
                Project path
              </label>
              <div className="flex gap-1.5">
                <input
                  type="text"
                  value={path}
                  onChange={(e) => setPath(e.target.value)}
                  onBlur={() => detect(path)}
                  placeholder="/home/you/code/my-project"
                  className="flex-1 px-2 py-1.5 text-xs font-mono bg-muted/20 border border-border rounded-md focus:outline-none focus:border-primary/40"
                />
                <button
                  onClick={browse}
                  className="px-2.5 py-1.5 rounded-md text-xs border border-border bg-muted/20 text-muted-foreground hover:text-foreground transition-colors"
                >
                  Browse…
                </button>
              </div>
            </div>
          ) : (
            <div className="flex flex-col gap-3">
              <div className="space-y-1.5">
                <label className="text-[9px] font-bold text-muted-foreground uppercase tracking-widest">
                  Repository URL
                </label>
                <input
                  type="text"
                  value={url}
                  onChange={(e) => setUrl(e.target.value)}
                  placeholder="https://github.com/user/repo.git"
                  className="w-full px-2 py-1.5 text-xs font-mono bg-muted/20 border border-border rounded-md focus:outline-none focus:border-primary/40"
                />
              </div>
              <div className="grid grid-cols-2 gap-1.5">
                <div className="space-y-1.5">
                  <label className="text-[9px] font-bold text-muted-foreground uppercase tracking-widest">
                    Name <span className="normal-case font-normal">(optional)</span>
                  </label>
                  <input
                    type="text"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="from URL"
                    className="w-full px-2 py-1.5 text-xs font-mono bg-muted/20 border border-border rounded-md focus:outline-none focus:border-primary/40"
                  />
                </div>
                <div className="space-y-1.5">
                  <label className="text-[9px] font-bold text-muted-foreground uppercase tracking-widest">
                    Branch <span className="normal-case font-normal">(optional)</span>
                  </label>
                  <input
                    type="text"
                    value={branch}
                    onChange={(e) => setBranch(e.target.value)}
                    placeholder="default"
                    className="w-full px-2 py-1.5 text-xs font-mono bg-muted/20 border border-border rounded-md focus:outline-none focus:border-primary/40"
                  />
                </div>
              </div>
            </div>
          )}

          {/* Category picker */}
          <div className="space-y-1.5">
            <label className="text-[9px] font-bold text-muted-foreground uppercase tracking-widest">
              Category
            </label>
            <div className="flex flex-wrap gap-1.5">
              <button
                onClick={() => setCategory("")}
                className={`flex items-center gap-1 px-2 py-1 rounded-md text-[10px] font-mono border transition-all ${
                  category === ""
                    ? "bg-primary/15 text-primary border-primary/40"
                    : "bg-muted/20 text-muted-foreground border-border hover:text-foreground"
                }`}
              >
                <Sparkles className="size-2.5" /> {autoLabel}
              </button>
              {categories.map((cat) => (
                <button
                  key={cat.id}
                  onClick={() => setCategory(cat.id)}
                  className={`px-2 py-1 rounded-md text-[10px] font-mono capitalize border transition-all ${
                    category === cat.id
                      ? "bg-primary/15 text-primary border-primary/40"
                      : "bg-muted/20 text-muted-foreground border-border hover:text-foreground"
                  }`}
                >
                  {cat.id}
                </button>
              ))}
            </div>
            {mode === "local" && detected && category === "" && (
              <p className="text-[10px] text-muted-foreground">
                Detected <span className="text-foreground font-mono">{detected.category}</span> — {detected.reason}
              </p>
            )}
          </div>

          <p className="text-[10px] text-muted-foreground">
            {mode === "local" ? "The folder will be moved" : "The repository will be cloned"} into{" "}
            <span className="font-mono">{base || "~/projects"}/{category || "<category>"}/</span>
          </p>

          {error && (
            <p className="text-[10px] text-rose-400 font-mono break-all">{error}</p>
          )}
        </div>

        {/* Footer */}
        <div className="p-3 border-t border-border/20 flex justify-end gap-2">
          <button
            onClick={onClose}
            disabled={busy}
            className="text-xs px-3 py-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            onClick={submit}
            disabled={busy || !canSubmit}
            className="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-md bg-primary hover:bg-primary/90 text-foreground font-semibold disabled:opacity-50 transition-all"
          >
            {busy && <Loader2 className="size-3 animate-spin" />}
            {busy ? (mode === "git" ? "Cloning…" : "Adding…") : "Add project"}
          </button>
        </div>
      </div>
    </div>
  );
}
