"use client";

import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Play,
  Square,
  RotateCw,
  TerminalSquare,
  Zap,
  Bot,
  ChevronDown,
  Settings2,
  ExternalLink,
  Plus,
  X,
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

const UI_LOG_CAP = 2000;

interface ShellTab {
  id: string;
  title: string;
}

/// Frontend-generated PTY session id, so the terminal component can subscribe
/// to the session's events before the shell is spawned.
function newSessionId(): string {
  return `t-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

const DOT: Record<Status, string> = {
  running: "bg-emerald-400",
  starting: "bg-amber-400 animate-pulse",
  stopped: "bg-muted",
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
        className="h-full w-full overflow-auto rounded-lg border border-border bg-background p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap text-foreground selection:bg-emerald-500/30"
      >
      {lines.length === 0 ? (
        <span className="text-muted-foreground">No output yet — press Start.</span>
      ) : (
        lines.map((l, i) => <div key={i}>{l === "" ? " " : l}</div>)
      )}
      </div>
      {copied && (
        <span className="pointer-events-none absolute right-2 top-2 rounded bg-emerald-600/90 px-1.5 py-0.5 text-[10px] font-medium text-foreground shadow">
          copied
        </span>
      )}
    </div>
  );
}

/// Dropdown that launches a configured AI agent CLI (Claude Code, Codex,
/// Gemini CLI, …) inside this project's shell session.
function AgentLauncher({
  onLaunch,
  onOpenSettings,
}: {
  onLaunch: (agent: AgentInfo) => Promise<void>;
  onOpenSettings?: (tab?: string) => void;
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
      // The panel opens a fresh shell tab and types the agent command into it.
      await onLaunch(agent);
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
        <div className="absolute right-0 top-full z-50 mt-1 w-64 rounded-lg border border-border bg-card p-1 shadow-xl">
          {agents.length === 0 ? (
            <div className="px-3 py-2 text-[11px] text-muted-foreground">
              No agents configured.
            </div>
          ) : (
            agents.map((agent) => (
              <button
                key={agent.name}
                disabled={!agent.installed || launching !== null}
                onClick={() => launch(agent)}
                className="flex w-full items-center gap-2 rounded px-2.5 py-1.5 text-left text-xs text-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-40"
                title={
                  agent.installed
                    ? `Run \`${agent.command}\` in the project shell`
                    : `'${agent.binary}' not found on $PATH`
                }
              >
                <span
                  className={`inline-block size-1.5 rounded-full ${
                    agent.installed ? "bg-emerald-400" : "bg-muted"
                  }`}
                />
                <span className="flex-1 truncate font-medium">
                  {launching === agent.name ? "Launching…" : agent.name}
                </span>
                <span className="max-w-[100px] truncate font-mono text-[10px] text-muted-foreground">
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
            <button
              type="button"
              onClick={() => {
                setOpen(false);
                onOpenSettings?.("agents");
              }}
              className="flex w-full items-center gap-2 rounded px-2.5 py-1.5 text-[11px] text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            >
              <Settings2 className="size-3" />
              Manage agents…
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export default function RunnerPanel({
  project,
  onOpenSettings,
}: {
  project: { name: string; path: string };
  onOpenSettings?: (tab?: string) => void;
}) {
  // The parent remounts this component (key={project.path}) when the project
  // changes, so state starts fresh each time — no in-effect resets needed.
  const [apps, setApps] = useState<RunnableApp[]>([]);
  const [statuses, setStatuses] = useState<Record<string, Status>>({});
  const [logs, setLogs] = useState<Record<string, string[]>>({});
  const [shellTabs, setShellTabs] = useState<ShellTab[]>(() => [
    { id: newSessionId(), title: "Shell" },
  ]);
  // Lazy initializer runs after the one above, so the first tab id is set.
  const [active, setActive] = useState<string>(() => shellTabs[0].id);
  const [discovery, setDiscovery] = useState<"loading" | "ready" | "empty" | "error">(
    "loading"
  );
  const [discoverError, setDiscoverError] = useState<string | null>(null);
  const [extTermError, setExtTermError] = useState<string | null>(null);
  // Incoming log lines are buffered here and flushed to state on animation frames
  // to keep re-renders bounded under heavy output.
  const pending = useRef<Record<string, string[]>>({});
  // Resolvers for tabs whose shell hasn't reported ready yet — agent launch
  // awaits these so it never types into a PTY that doesn't exist.
  const readyWaiters = useRef<Record<string, () => void>>({});
  const shellCounter = useRef(1);
  // Mirror of shellTabs for callbacks that fire long after their render
  // (agent-launch failure, terminal-exit events).
  const shellTabsRef = useRef<ShellTab[]>([]);

  useEffect(() => {
    shellTabsRef.current = shellTabs;
  }, [shellTabs]);

  const markTabReady = (id: string) => {
    readyWaiters.current[id]?.();
    delete readyWaiters.current[id];
  };

  const addShellTab = () => {
    shellCounter.current += 1;
    const tab = { id: newSessionId(), title: `Shell ${shellCounter.current}` };
    setShellTabs((tabs) => [...tabs, tab]);
    setActive(tab.id);
    return tab;
  };

  const removeShellTab = (id: string, killBackend: boolean) => {
    if (killBackend) {
      invoke("cmd_kill_terminal", { id }).catch(() => {});
    }
    delete readyWaiters.current[id];
    const next = shellTabsRef.current.filter((t) => t.id !== id);
    setShellTabs(next);
    setActive((a) =>
      a === id ? next[next.length - 1]?.id ?? apps[0]?.id ?? "" : a
    );
  };

  // Open a fresh tab for the agent, wait until its shell is live, then type
  // the agent command into it. Errors propagate to the launcher dropdown.
  const launchAgent = async (agent: AgentInfo) => {
    shellCounter.current += 1;
    const id = newSessionId();
    const ready = new Promise<void>((resolve) => {
      readyWaiters.current[id] = resolve;
    });
    setShellTabs((tabs) => [...tabs, { id, title: agent.name }]);
    setActive(id);
    try {
      await Promise.race([
        ready,
        new Promise<never>((_, reject) =>
          setTimeout(() => reject(new Error("terminal did not start")), 8000)
        ),
      ]);
      await invoke("cmd_launch_agent", { name: agent.name, sessionId: id });
    } catch (e) {
      removeShellTab(id, true);
      throw e;
    }
  };

  const openExternalTerminal = () => {
    setExtTermError(null);
    invoke("cmd_open_external_terminal", { path: project.path }).catch((e) => {
      setExtTermError(String(e));
      window.setTimeout(() => setExtTermError(null), 6000);
    });
  };

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
          {shellTabs.map((t) => (
            <TabsTrigger key={t.id} value={t.id}>
              <TerminalSquare className="size-3" />
              {t.title}
              <span
                role="button"
                tabIndex={-1}
                title="Close tab"
                onClick={(e) => {
                  e.stopPropagation();
                  removeShellTab(t.id, true);
                }}
                className="ml-1 rounded p-0.5 text-muted-foreground hover:bg-accent hover:text-foreground"
              >
                <X className="size-2.5" />
              </span>
            </TabsTrigger>
          ))}
          <button
            type="button"
            onClick={addShellTab}
            title="New shell tab"
            className="mx-0.5 rounded p-1 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            <Plus className="size-3" />
          </button>
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
                <span className="text-muted-foreground">discovering apps…</span>
              )}
              {discovery === "empty" && (
                <span className="text-muted-foreground">no runnable apps detected here</span>
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
          {extTermError && (
            <span
              className="max-w-[220px] truncate font-mono text-[11px] text-amber-400"
              title={extTermError}
            >
              {extTermError}
            </span>
          )}
          <Button
            size="xs"
            variant="outline"
            title="Open this project in your external terminal"
            onClick={openExternalTerminal}
          >
            <ExternalLink className="size-3" />
            Terminal
          </Button>
          <AgentLauncher onLaunch={launchAgent} onOpenSettings={onOpenSettings} />
        </div>
      </div>

      {shellTabs.map((t) => (
        <TabsContent key={t.id} value={t.id} keepMounted className="min-h-0">
          <div className="h-full w-full">
            <TerminalView
              cwd={project.path}
              sessionId={t.id}
              onReady={() => markTabReady(t.id)}
              onExit={() => removeShellTab(t.id, false)}
            />
          </div>
        </TabsContent>
      ))}

      {apps.map((a) => (
        <TabsContent key={a.id} value={a.id} keepMounted className="min-h-0">
          <div className="flex h-full min-h-0 flex-col gap-2">
            <div className="flex shrink-0 flex-wrap items-center justify-between gap-2">
              <div className="flex items-center gap-2 font-mono text-xs">
                <StatusDot status={statuses[a.id]} />
                <span className="text-foreground">{a.label}</span>
                {a.port ? <span className="text-muted-foreground">:{a.port}</span> : null}
                <span className="text-muted-foreground">· {a.command}</span>
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
