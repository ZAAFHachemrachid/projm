"use client";

import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Link from "next/link";
import {
  Play,
  Square,
  RotateCw,
  TerminalSquare,
  Zap,
  Bot,
  ChevronDown,
  Settings2,
} from "lucide-react";

import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import TerminalView from "@/components/ui/terminal";

type Status = "stopped" | "starting" | "running" | "errored";

interface RunnableApp {
  id: string;
  label: string;
  dir: string;
  command: string;
  port: number | null;
  hint: string;
  status: Status;
  pid: number | null;
}

interface AgentInfo {
  name: string;
  command: string;
  binary: string;
  installed: boolean;
  path: string | null;
}

interface LogEvent {
  project: string;
  app_id: string;
  lines: string[];
}
interface StatusEvent {
  project: string;
  app_id: string;
  status: Status;
  pid: number | null;
}

const SHELL = "__shell__";
const UI_LOG_CAP = 2000;

const DOT: Record<Status, string> = {
  running: "bg-emerald-400",
  starting: "bg-amber-400 animate-pulse",
  stopped: "bg-slate-600",
  errored: "bg-red-500",
};

function StatusDot({ status }: { status?: Status }) {
  return (
    <span
      className={`inline-block size-2 rounded-full ${DOT[status ?? "stopped"]}`}
      aria-label={status ?? "stopped"}
    />
  );
}

function LogView({ lines }: { lines: string[] }) {
  const ref = useRef<HTMLDivElement>(null);
  const [copied, setCopied] = useState(false);

  // Auto-copy whatever the user selects in the log to the clipboard.
  const copySelection = () => {
    const sel = window.getSelection?.()?.toString() ?? "";
    if (!sel.trim()) return;
    navigator.clipboard
      ?.writeText(sel)
      .then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1200);
      })
      .catch(() => {});
  };

  useEffect(() => {
    const el = ref.current;
    if (el) el.scrollTop = el.scrollHeight;
  });
  return (
    <div className="relative h-full w-full">
      <div
        ref={ref}
        onMouseUp={copySelection}
        className="h-full w-full overflow-auto rounded-lg border border-border bg-[#0c0d0e] p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap text-slate-200 selection:bg-emerald-500/30"
      >
      {lines.length === 0 ? (
        <span className="text-slate-500">No output yet — press Start.</span>
      ) : (
        lines.map((l, i) => <div key={i}>{l === "" ? " " : l}</div>)
      )}
      </div>
      {copied && (
        <span className="pointer-events-none absolute right-2 top-2 rounded bg-emerald-600/90 px-1.5 py-0.5 text-[10px] font-medium text-white shadow">
          copied
        </span>
      )}
    </div>
  );
}

/// Dropdown that launches a configured AI agent CLI (Claude Code, Codex,
/// Gemini CLI, …) inside this project's shell session.
function AgentLauncher({
  cwd,
  onLaunch,
}: {
  cwd: string;
  onLaunch: () => void;
}) {
  const [agents, setAgents] = useState<AgentInfo[]>([]);
  const [open, setOpen] = useState(false);
  const [launching, setLaunching] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let alive = true;
    invoke<AgentInfo[]>("cmd_get_agents")
      .then((list) => alive && setAgents(list))
      .catch((e) => console.error("failed to load AI agents", e));
    return () => {
      alive = false;
    };
  }, []);

  // Close on any outside click.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    window.addEventListener("mousedown", onDown);
    return () => window.removeEventListener("mousedown", onDown);
  }, [open]);

  const launch = async (agent: AgentInfo) => {
    setLaunching(agent.name);
    setError(null);
    try {
      // Show the shell first so the user watches the agent boot in it.
      onLaunch();
      await invoke("cmd_launch_agent", { cwd, name: agent.name });
      setOpen(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setLaunching(null);
    }
  };

  return (
    <div ref={rootRef} className="relative shrink-0">
      <Button
        size="xs"
        variant="outline"
        onClick={() => setOpen((o) => !o)}
        title="Launch an AI coding agent in this project's shell"
      >
        <Bot className="size-3 text-emerald-400" />
        AI Agent
        <ChevronDown className="size-3 opacity-60" />
      </Button>

      {open && (
        <div className="absolute right-0 top-full z-50 mt-1 w-64 rounded-lg border border-border bg-[#101113] p-1 shadow-xl">
          {agents.length === 0 ? (
            <div className="px-3 py-2 text-[11px] text-slate-500">
              No agents configured.
            </div>
          ) : (
            agents.map((agent) => (
              <button
                key={agent.name}
                disabled={!agent.installed || launching !== null}
                onClick={() => launch(agent)}
                className="flex w-full items-center gap-2 rounded px-2.5 py-1.5 text-left text-xs text-slate-200 transition-colors hover:bg-white/5 disabled:cursor-not-allowed disabled:opacity-40"
                title={
                  agent.installed
                    ? `Run \`${agent.command}\` in the project shell`
                    : `'${agent.binary}' not found on $PATH`
                }
              >
                <span
                  className={`inline-block size-1.5 rounded-full ${
                    agent.installed ? "bg-emerald-400" : "bg-slate-600"
                  }`}
                />
                <span className="flex-1 truncate font-medium">
                  {launching === agent.name ? "Launching…" : agent.name}
                </span>
                <span className="max-w-[100px] truncate font-mono text-[10px] text-slate-500">
                  {agent.command}
                </span>
              </button>
            ))
          )}
          {error && (
            <div className="mx-1 my-1 rounded border border-red-500/20 bg-red-500/10 px-2 py-1.5 text-[10px] text-red-400">
              {error}
            </div>
          )}
          <div className="mt-1 border-t border-border/40 pt-1">
            <Link
              href="/settings?tab=agents"
              className="flex items-center gap-2 rounded px-2.5 py-1.5 text-[11px] text-slate-400 transition-colors hover:bg-white/5 hover:text-white"
            >
              <Settings2 className="size-3" />
              Manage agents…
            </Link>
          </div>
        </div>
      )}
    </div>
  );
}

export default function RunnerPanel({
  project,
}: {
  project: { name: string; path: string };
}) {
  // The parent remounts this component (key={project.path}) when the project
  // changes, so state starts fresh each time — no in-effect resets needed.
  const [apps, setApps] = useState<RunnableApp[]>([]);
  const [statuses, setStatuses] = useState<Record<string, Status>>({});
  const [logs, setLogs] = useState<Record<string, string[]>>({});
  const [active, setActive] = useState<string>(SHELL);
  const [discovery, setDiscovery] = useState<"loading" | "ready" | "empty" | "error">(
    "loading"
  );
  const [discoverError, setDiscoverError] = useState<string | null>(null);
  // Incoming log lines are buffered here and flushed to state on animation frames
  // to keep re-renders bounded under heavy output.
  const pending = useRef<Record<string, string[]>>({});

  // Discover runnable apps for the selected project.
  useEffect(() => {
    let alive = true;
    invoke<RunnableApp[]>("cmd_runner_discover", { projectPath: project.path })
      .then((list) => {
        if (!alive) return;
        const s: Record<string, Status> = {};
        list.forEach((a) => {
          s[a.id] = a.status;
        });
        setApps(list);
        setStatuses(s);
        setDiscovery(list.length === 0 ? "empty" : "ready");
      })
      .catch((e) => {
        if (!alive) return;
        console.error("runner discover failed", e);
        setDiscovery("error");
        setDiscoverError(String(e));
      });
    return () => {
      alive = false;
    };
  }, [project.path]);

  // Stream log lines + status changes from the backend.
  useEffect(() => {
    // `listen` is async; if this effect is torn down (e.g. StrictMode's
    // mount→cleanup→remount in dev) before the promise resolves, dispose the
    // listener the moment it arrives — otherwise it leaks and every log line
    // is delivered twice.
    let disposed = false;
    let uLog: (() => void) | null = null;
    let uStat: (() => void) | null = null;
    let raf = 0;
    const flush = () => {
      raf = 0;
      const buf = pending.current;
      pending.current = {};
      setLogs((prev) => {
        const next = { ...prev };
        for (const [id, incoming] of Object.entries(buf)) {
          const arr = (next[id] ?? []).concat(incoming);
          next[id] =
            arr.length > UI_LOG_CAP ? arr.slice(arr.length - UI_LOG_CAP) : arr;
        }
        return next;
      });
    };
    const schedule = () => {
      if (raf === 0) raf = requestAnimationFrame(flush);
    };

    listen<LogEvent>("runner:log", (e) => {
      const p = e.payload;
      if (p.project !== project.path) return;
      (pending.current[p.app_id] ??= []).push(...p.lines);
      schedule();
    }).then((f) => (disposed ? f() : (uLog = f)));

    listen<StatusEvent>("runner:status", (e) => {
      const p = e.payload;
      if (p.project !== project.path) return;
      setStatuses((s) => ({ ...s, [p.app_id]: p.status }));
    }).then((f) => (disposed ? f() : (uStat = f)));

    return () => {
      disposed = true;
      if (uLog) uLog();
      if (uStat) uStat();
      if (raf) cancelAnimationFrame(raf);
    };
  }, [project.path]);

  const call = (cmd: string, appId?: string) =>
    invoke(cmd, {
      projectPath: project.path,
      ...(appId ? { appId } : {}),
    }).catch((e) => console.error(cmd, e));

  return (
    <Tabs
      value={active}
      onValueChange={(v) => setActive(String(v))}
      className="flex h-full min-h-0 flex-1 flex-col"
    >
      <div className="mb-2 flex shrink-0 items-center justify-between gap-2">
        <TabsList variant="line" className="flex-wrap">
          <TabsTrigger value={SHELL}>
            <TerminalSquare className="size-3" />
            Shell
          </TabsTrigger>
          {apps.map((a) => (
            <TabsTrigger key={a.id} value={a.id}>
              <StatusDot status={statuses[a.id]} />
              <span className="ml-1">{a.label}</span>
              {a.port ? (
                <span className="ml-1 text-muted-foreground">:{a.port}</span>
              ) : null}
            </TabsTrigger>
          ))}
        </TabsList>
        <div className="flex shrink-0 items-center gap-1">
          {apps.length > 0 ? (
            <>
              <Button size="xs" variant="outline" onClick={() => call("cmd_runner_start_all")}>
                <Play className="size-3" />
                Start all
              </Button>
              <Button size="xs" variant="outline" onClick={() => call("cmd_runner_stop_all")}>
                <Square className="size-3" />
                Stop all
              </Button>
            </>
          ) : (
            <div className="mr-1 font-mono text-[11px]">
              {discovery === "loading" && (
                <span className="text-slate-500">discovering apps…</span>
              )}
              {discovery === "empty" && (
                <span className="text-slate-500">no runnable apps detected here</span>
              )}
              {discovery === "error" && (
                <span
                  className="text-amber-400"
                  title={discoverError ?? undefined}
                >
                  runner backend unavailable — rebuild &amp; restart the desktop app
                </span>
              )}
            </div>
          )}
          <AgentLauncher cwd={project.path} onLaunch={() => setActive(SHELL)} />
        </div>
      </div>

      <TabsContent value={SHELL} keepMounted className="min-h-0">
        <div className="h-full w-full">
          <TerminalView cwd={project.path} />
        </div>
      </TabsContent>

      {apps.map((a) => (
        <TabsContent key={a.id} value={a.id} keepMounted className="min-h-0">
          <div className="flex h-full min-h-0 flex-col gap-2">
            <div className="flex shrink-0 flex-wrap items-center justify-between gap-2">
              <div className="flex items-center gap-2 font-mono text-xs">
                <StatusDot status={statuses[a.id]} />
                <span className="text-slate-200">{a.label}</span>
                {a.port ? <span className="text-slate-500">:{a.port}</span> : null}
                <span className="text-slate-600">· {a.command}</span>
              </div>
              <div className="flex items-center gap-1">
                <Button size="xs" onClick={() => call("cmd_runner_start", a.id)}>
                  <Play className="size-3" />
                  Start
                </Button>
                <Button size="xs" variant="outline" onClick={() => call("cmd_runner_stop", a.id)}>
                  <Square className="size-3" />
                  Stop
                </Button>
                <Button
                  size="xs"
                  variant="secondary"
                  onClick={() => call("cmd_runner_restart", a.id)}
                >
                  <RotateCw className="size-3" />
                  Restart
                </Button>
                {a.port ? (
                  <Button
                    size="xs"
                    variant="ghost"
                    title={`Kill whatever is using port ${a.port}`}
                    onClick={() => call("cmd_runner_free_port", a.id)}
                  >
                    <Zap className="size-3" />
                    Free :{a.port}
                  </Button>
                ) : null}
              </div>
            </div>
            <div className="min-h-0 flex-1">
              <LogView lines={logs[a.id] ?? []} />
            </div>
          </div>
        </TabsContent>
      ))}
    </Tabs>
  );
}
