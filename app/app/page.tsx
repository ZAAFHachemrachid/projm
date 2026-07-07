"use client";

import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useHotkey } from "@tanstack/react-hotkeys";
import {
  FolderTree,
  Terminal as TerminalIcon,
  Settings,
  Search,
  RefreshCw,
  GitBranch,
  FolderPlus,
  FilePlus,
  Play,
  HeartPulse,
  Settings2,
  FolderOpen,
  Info,
  ChevronRight,
  Sparkles,
  Layers,
  Activity,
  Tag
} from "lucide-react";
import RunnerPanel from "@/components/ui/runner-panel";
import ProjectTabs from "@/components/project-tabs";
import { SettingsPanel } from "@/components/settings-panel";
import { ScanPanel } from "@/components/scan-panel";
import { AddProjectDialog } from "@/components/add-project-dialog";
import {
  TooltipProvider,
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/components/ui/tooltip";

interface ProjectItem {
  name: String;
  path: String;
  category: string;
  git_branch?: string;
  git_dirty?: boolean;
}

interface Config {
  base: string;
  categories?: string[];
}

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  git_status: string | null;
}

const CATEGORY_META: Record<string, {
  letter: string;
  name: string;
  color: string;
  bg: string;
  border: string;
  hoverBg: string;
  progressColor: string;
  accentHex: string;
}> = {
  apps: {
    letter: "A",
    name: "Apps",
    color: "text-purple-400",
    bg: "bg-purple-500/10",
    border: "border-purple-500/20",
    hoverBg: "hover:bg-purple-500/15",
    progressColor: "bg-purple-500",
    accentHex: "#a855f7",
  },
  services: {
    letter: "S",
    name: "Services",
    color: "text-blue-400",
    bg: "bg-blue-500/10",
    border: "border-blue-500/20",
    hoverBg: "hover:bg-blue-500/15",
    progressColor: "bg-blue-500",
    accentHex: "#3b82f6",
  },
  ui: {
    letter: "U",
    name: "UI Components",
    color: "text-emerald-400",
    bg: "bg-emerald-500/10",
    border: "border-emerald-500/20",
    hoverBg: "hover:bg-emerald-500/15",
    progressColor: "bg-emerald-500",
    accentHex: "#10b981",
  },
  embedded: {
    letter: "E",
    name: "Embedded",
    color: "text-cyan-400",
    bg: "bg-cyan-500/10",
    border: "border-cyan-500/20",
    hoverBg: "hover:bg-cyan-500/15",
    progressColor: "bg-cyan-500",
    accentHex: "#06b6d4",
  },
  ml: {
    letter: "M",
    name: "Machine Learning",
    color: "text-amber-400",
    bg: "bg-amber-500/10",
    border: "border-amber-500/20",
    hoverBg: "hover:bg-amber-500/15",
    progressColor: "bg-amber-500",
    accentHex: "#f59e0b",
  },
  tools: {
    letter: "T",
    name: "CLI Tools",
    color: "text-orange-400",
    bg: "bg-orange-500/10",
    border: "border-orange-500/20",
    hoverBg: "hover:bg-orange-500/15",
    progressColor: "bg-orange-500",
    accentHex: "#f97316",
  },
  labs: {
    letter: "L",
    name: "Labs",
    color: "text-rose-400",
    bg: "bg-rose-500/10",
    border: "border-rose-500/20",
    hoverBg: "hover:bg-rose-500/15",
    progressColor: "bg-rose-500",
    accentHex: "#f43f5e",
  },
  content: {
    letter: "C",
    name: "Content Docs",
    color: "text-pink-400",
    bg: "bg-pink-500/10",
    border: "border-pink-500/20",
    hoverBg: "hover:bg-pink-500/15",
    progressColor: "bg-pink-500",
    accentHex: "#ec4899",
  },
};

const CUSTOM_PALETTES = [
  {
    color: "text-violet-400",
    bg: "bg-violet-500/10",
    border: "border-violet-500/20",
    hoverBg: "hover:bg-violet-500/15",
    progressColor: "bg-violet-500",
    accentHex: "#8b5cf6",
  },
  {
    color: "text-teal-400",
    bg: "bg-teal-500/10",
    border: "border-teal-500/20",
    hoverBg: "hover:bg-teal-500/15",
    progressColor: "bg-teal-500",
    accentHex: "#14b8a6",
  },
  {
    color: "text-fuchsia-400",
    bg: "bg-fuchsia-500/10",
    border: "border-fuchsia-500/20",
    hoverBg: "hover:bg-fuchsia-500/15",
    progressColor: "bg-fuchsia-500",
    accentHex: "#d946ef",
  },
  {
    color: "text-sky-400",
    bg: "bg-sky-500/10",
    border: "border-sky-500/20",
    hoverBg: "hover:bg-sky-500/15",
    progressColor: "bg-sky-500",
    accentHex: "#0ea5e9",
  },
  {
    color: "text-lime-400",
    bg: "bg-lime-500/10",
    border: "border-lime-500/20",
    hoverBg: "hover:bg-lime-500/15",
    progressColor: "bg-lime-500",
    accentHex: "#84cc16",
  },
];

function getCategoryMeta(catId: string, index: number = 0) {
  const normalized = catId.toLowerCase().trim();
  if (CATEGORY_META[normalized]) {
    return CATEGORY_META[normalized];
  }
  const palette = CUSTOM_PALETTES[index % CUSTOM_PALETTES.length];
  return {
    letter: normalized.charAt(0).toUpperCase() || "C",
    name: normalized.charAt(0).toUpperCase() + normalized.slice(1),
    ...palette,
  };
}

function getStackTags(project: ProjectItem): string[] {
  const name = project.name.toLowerCase();
  const cat = project.category.toLowerCase();
  
  if (name.includes("react") || name.includes("next") || name.includes("ui") || cat === "ui") {
    return ["React", "TypeScript", "Tailwind"];
  }
  if (name.includes("rust") || name.includes("tauri") || name.includes("cargo")) {
    return ["Rust", "Tauri"];
  }
  if (cat === "ml" || name.includes("py") || name.includes("learn") || name.includes("model")) {
    return ["Python", "PyTorch", "Jupyter"];
  }
  if (cat === "embedded" || name.includes("firmware") || name.includes("arduino") || name.includes("esp")) {
    return ["C++", "PlatformIO", "FreeRTOS"];
  }
  if (cat === "tools" || name.includes("cli") || name.includes("go")) {
    return ["Go", "Cobra CLI"];
  }
  if (cat === "services" || name.includes("api") || name.includes("db") || name.includes("server")) {
    return ["Node.js", "Express", "Postgres"];
  }
  if (cat === "content" || name.includes("docs") || name.includes("blog")) {
    return ["MDX", "Astro", "Tailwind"];
  }
  return ["TypeScript", "Node.js"];
}

// VS Code Explorer-like File Tree Node Component
function FileTreeNode({
  entry,
  depth,
  expandedPaths,
  loadedChildren,
  onToggleExpand,
}: {
  entry: FileEntry;
  depth: number;
  expandedPaths: Set<string>;
  loadedChildren: Record<string, FileEntry[]>;
  onToggleExpand: (path: string) => void;
}) {
  const isExpanded = expandedPaths.has(entry.path);
  const children = loadedChildren[entry.path] || [];

  const getFileIcon = () => {
    if (entry.is_dir) {
      return isExpanded 
        ? <FolderOpen className="size-3.5 text-primary shrink-0" />
        : <FolderOpen className="size-3.5 text-muted-foreground shrink-0" />;
    }

    const name = entry.name.toLowerCase();
    if (name.endsWith(".rs")) {
      return <span className="text-orange-500 font-bold font-mono text-[10px] size-3.5 shrink-0 flex items-center justify-center">🦀</span>;
    }
    if (name.endsWith(".md")) {
      return <span className="text-blue-400 font-bold font-mono text-[9px] size-3.5 shrink-0 flex items-center justify-center bg-blue-500/10 border border-blue-500/20 rounded px-0.5">M↓</span>;
    }
    if (name.endsWith(".json") || name.endsWith(".toml") || name.endsWith(".yaml") || name.endsWith(".yml")) {
      return <span className="text-yellow-500 font-bold font-mono text-[10px] size-3.5 shrink-0 flex items-center justify-center">⚙️</span>;
    }
    if (name === ".gitignore" || name.startsWith(".git")) {
      return <GitBranch className="size-3.5 text-muted-foreground shrink-0" />;
    }
    
    return <span className="text-muted-foreground font-bold font-mono text-[10px] size-3.5 shrink-0 flex items-center justify-center">📄</span>;
  };

  const getGitStyle = () => {
    if (!entry.git_status) return null;
    const status = entry.git_status.toLowerCase();
    
    if (status.includes("?") || status.includes("u")) {
      return { label: "U", colorClass: "text-emerald-400" };
    }
    if (status.includes("m")) {
      return { label: "M", colorClass: "text-amber-400 animate-pulse" };
    }
    if (status.includes("d")) {
      return { label: "D", colorClass: "text-rose-400" };
    }
    if (status.includes("a")) {
      return { label: "A", colorClass: "text-[#10b981]" };
    }
    return { label: entry.git_status, colorClass: "text-muted-foreground" };
  };

  const gitStyle = getGitStyle();

  return (
    <div className="flex flex-col w-full select-none">
      <button
        onClick={() => entry.is_dir && onToggleExpand(entry.path)}
        className="w-full flex items-center justify-between py-1 px-2 rounded-md hover:bg-card/60 group text-left transition-colors relative"
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
      >
        {/* Indenting Guide Lines */}
        {depth > 0 && Array.from({ length: depth }).map((_, i) => (
          <span 
            key={i} 
            className="absolute top-0 bottom-0 border-l border-border" 
            style={{ left: `${i * 12 + 14}px` }} 
          />
        ))}

        <div className="flex items-center gap-2 truncate">
          {entry.is_dir && (
            <ChevronRight className={`size-3 text-muted-foreground transition-transform duration-150 shrink-0 ${isExpanded ? "rotate-90" : ""}`} />
          )}
          {!entry.is_dir && <span className="w-3 shrink-0" />}
          
          {getFileIcon()}
          
          <span className={`text-[11px] font-mono truncate ${gitStyle ? gitStyle.colorClass : "text-foreground group-hover:text-foreground"}`}>
            {entry.name}
          </span>
        </div>

        {gitStyle && (
          <span className={`text-[10px] font-bold font-mono px-1.5 ${gitStyle.colorClass}`}>
            {gitStyle.label}
          </span>
        )}
      </button>

      {entry.is_dir && isExpanded && children.length > 0 && (
        <div className="flex flex-col">
          {children.map((child) => (
            <FileTreeNode
              key={child.path}
              entry={child}
              depth={depth + 1}
              expandedPaths={expandedPaths}
              loadedChildren={loadedChildren}
              onToggleExpand={onToggleExpand}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// Collapsible Accordion Panel Component (VS Code styled)
interface CollapsiblePanelProps {
  title: string;
  isExpanded: boolean;
  onToggle: () => void;
  paddingClass?: string;
  children: React.ReactNode;
}

function CollapsiblePanel({
  title,
  isExpanded,
  onToggle,
  paddingClass = "px-2 py-2",
  children,
}: CollapsiblePanelProps) {
  return (
    <div className="flex flex-col w-full border-b border-border">
      <button
        onClick={onToggle}
        className="w-full flex items-center justify-between py-1.5 px-3 bg-background/80 hover:bg-card/90 transition-colors group text-left select-none outline-none focus:outline-none border-t border-border"
      >
        <div className="flex items-center gap-1.5">
          <ChevronRight 
            className={`size-3 text-muted-foreground group-hover:text-foreground transition-transform duration-150 shrink-0 ${
              isExpanded ? "rotate-90" : ""
            }`} 
          />
          <span className="text-[11px] font-bold tracking-wide text-muted-foreground group-hover:text-foreground font-sans">
            {title}
          </span>
        </div>
      </button>
      
      {isExpanded && (
        <div className={`${paddingClass} flex flex-col gap-0.5 w-full bg-card`}>
          {children}
        </div>
      )}
    </div>
  );
}

export default function WorkspacePage() {
  // Navigation & Category states
  const [selectedCategory, setSelectedCategory] = useState("apps");
  const [categories, setCategories] = useState<{ id: string; name: string }[]>([
    { id: "apps", name: "Apps" },
    { id: "services", name: "Services" },
    { id: "ui", name: "UI Components" },
    { id: "embedded", name: "Embedded" },
    { id: "ml", name: "Machine Learning" },
    { id: "tools", name: "CLI Tools" },
    { id: "labs", name: "Labs" },
    { id: "content", name: "Content Docs" },
  ]);
  const [projects, setProjects] = useState<ProjectItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [config, setConfig] = useState<Config | null>(null);

  // Active workspace states
  const [selectedProject, setSelectedProject] = useState<ProjectItem | null>(null);
  const [activeTab, setActiveTab] = useState<"projects" | "diagnostics">("projects");
  // Ordered working set of opened projects (the tab strip), plus which of them
  // have been activated at least once — only those mount a RunnerPanel, so
  // restored-but-never-visited tabs don't spawn shells at startup.
  const [openProjects, setOpenProjects] = useState<ProjectItem[]>([]);
  const [activatedPaths, setActivatedPaths] = useState<Set<string>>(new Set());
  // project path → app_id → runner status, fed by the global runner:status
  // stream; drives the per-tab session dot.
  const [runnerActivity, setRunnerActivity] = useState<Record<string, Record<string, string>>>({});
  const tabsRestoredRef = useRef(false);
  const [diagnosticText, setDiagnosticText] = useState("");
  const [runningDiagnostics, setRunningDiagnostics] = useState(false);

  // Search & interaction states
  const [searchTerm, setSearchTerm] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [scanning, setScanning] = useState(false);
  // In-page overlays for Settings / Scan, rendered ON TOP of the workspace so
  // every RunnerPanel (and its live terminals) stays mounted underneath.
  // Navigating to the /settings or /scan routes would unmount the workspace and
  // wipe all shell sessions — this keeps them alive.
  const [overlay, setOverlay] = useState<
    { kind: "settings"; tab?: string } | { kind: "scan" } | null
  >(null);
  const [showAddProject, setShowAddProject] = useState(false);
  const [categoriesExpanded, setCategoriesExpanded] = useState(true);
  const [projectsExpanded, setProjectsExpanded] = useState(true);

  // File tree explorer states
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());
  const [loadedChildren, setLoadedChildren] = useState<Record<string, FileEntry[]>>({});
  const [explorerExpanded, setExplorerExpanded] = useState(true);
  const [outlineExpanded, setOutlineExpanded] = useState(false);
  const [timelineExpanded, setTimelineExpanded] = useState(false);
  const [fileTreeProExpanded, setFileTreeProExpanded] = useState(false);

  // Load backend configurations and projects list
  // ── Assign-category dialog state ──────────────────────────────────────────
  const [assignTarget, setAssignTarget] = useState<ProjectItem | null>(null);
  const [assignCategory, setAssignCategory] = useState("");
  const [assignMode, setAssignMode] = useState<"marker" | "rule">("marker");
  const [assignMove, setAssignMove] = useState(true);
  const [assignBusy, setAssignBusy] = useState(false);
  const [assignError, setAssignError] = useState<string | null>(null);

  function openAssignDialog(p: ProjectItem) {
    setAssignTarget(p);
    setAssignCategory(p.category);
    setAssignMode("marker");
    setAssignMove(true);
    setAssignError(null);
  }

  async function handleAssignCategory() {
    if (!assignTarget || !assignCategory) return;
    setAssignBusy(true);
    setAssignError(null);
    try {
      await invoke<string | null>("cmd_assign_category", {
        path: assignTarget.path.toString(),
        category: assignCategory,
        mode: assignMode,
        group: null,
        moveProject: assignMove,
      });
      setAssignTarget(null);
      await loadData();
    } catch (err) {
      setAssignError(`${err}`);
    } finally {
      setAssignBusy(false);
    }
  }

  async function loadData() {
    setLoading(true);
    try {
      const cfg = await invoke<Config>("cmd_get_config");
      setConfig(cfg);
      if (cfg.categories && Array.from(cfg.categories).length > 0) {
        const formattedCats = cfg.categories.map((c) => ({
          id: c.toLowerCase(),
          name: c.charAt(0).toUpperCase() + c.slice(1),
        }));
        setCategories(formattedCats);
        if (formattedCats.length > 0 && !formattedCats.some(cat => cat.id === selectedCategory)) {
          setSelectedCategory(formattedCats[0].id);
        }
      }
      const prjs = await invoke<ProjectItem[]>("cmd_list_projects");
      setProjects(prjs);
    } catch (err) {
      console.error("Failed to load workspace data", err);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadData();
  }, []);

  // Open a project as a tab (deduped by path) and make it the active one.
  function openProject(p: ProjectItem) {
    const path = p.path.toString();
    setOpenProjects((prev) =>
      prev.some((x) => x.path.toString() === path) ? prev : [...prev, p]
    );
    setActivatedPaths((prev) => {
      if (prev.has(path)) return prev;
      const next = new Set(prev);
      next.add(path);
      return next;
    });
    setSelectedProject(p);
    setActiveTab("projects");
  }

  // Close a tab: tear down its shell + runner sessions, then activate a
  // neighbor if the closed tab was the active one.
  function closeProject(path: string) {
    invoke("cmd_kill_project_terminals", { cwd: path }).catch(() => {});
    invoke("cmd_runner_stop_all", { projectPath: path }).catch(() => {});
    const idx = openProjects.findIndex((p) => p.path.toString() === path);
    const next = openProjects.filter((p) => p.path.toString() !== path);
    setOpenProjects(next);
    setActivatedPaths((prev) => {
      const s = new Set(prev);
      s.delete(path);
      return s;
    });
    setRunnerActivity((prev) => {
      if (!(path in prev)) return prev;
      const n = { ...prev };
      delete n[path];
      return n;
    });
    if (selectedProject?.path.toString() === path) {
      setSelectedProject(
        next.length > 0 ? next[Math.min(Math.max(idx, 0), next.length - 1)] : null
      );
    }
  }

  // Cycle the active tab forward/backward (Ctrl+Tab / Ctrl+Shift+Tab).
  function cycleTab(dir: 1 | -1) {
    if (openProjects.length === 0) return;
    const idx = openProjects.findIndex(
      (p) => p.path.toString() === selectedProject?.path.toString()
    );
    const nextIdx =
      idx === -1
        ? dir === 1
          ? 0
          : openProjects.length - 1
        : (idx + dir + openProjects.length) % openProjects.length;
    openProject(openProjects[nextIdx]);
  }

  // Bind professional cross-platform hotkeys using TanStack React Hotkeys
  useHotkey("Mod+K", (e) => {
    e.preventDefault();
    setSearchOpen((prev) => !prev);
  });

  useHotkey("Control+Tab" as any, (e: KeyboardEvent) => {
    e.preventDefault();
    cycleTab(1);
  });

  useHotkey("Control+Shift+Tab" as any, (e: KeyboardEvent) => {
    e.preventDefault();
    cycleTab(-1);
  });

  // Fallback bindings — some webviews swallow Ctrl+Tab before it reaches JS.
  useHotkey("Control+PageDown" as any, (e: KeyboardEvent) => {
    e.preventDefault();
    cycleTab(1);
  });

  useHotkey("Control+PageUp" as any, (e: KeyboardEvent) => {
    e.preventDefault();
    cycleTab(-1);
  });

  useHotkey("Mod+W" as any, (e: KeyboardEvent) => {
    e.preventDefault();
    if (selectedProject) closeProject(selectedProject.path.toString());
  });

  // Global runner activity stream → per-tab session dots. Panels stay mounted
  // per open project, so status events flow for every tab, not just the
  // visible one.
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    listen<{ project: string; app_id: string; status: string }>(
      "runner:status",
      (e) => {
        const { project, app_id, status } = e.payload;
        setRunnerActivity((prev) => ({
          ...prev,
          [project]: { ...prev[project], [app_id]: status },
        }));
      }
    ).then((f) => (disposed ? f() : (unlisten = f)));
    return () => {
      disposed = true;
      if (unlisten) unlisten();
    };
  }, []);

  // Restore the open-tab set from localStorage once projects are loaded.
  useEffect(() => {
    if (tabsRestoredRef.current || loading || projects.length === 0) return;
    tabsRestoredRef.current = true;
    try {
      const raw = localStorage.getItem("projm.openTabs");
      if (!raw) return;
      const saved = JSON.parse(raw) as { paths?: string[]; active?: string | null };
      if (!saved.paths?.length) return;
      const found = saved.paths
        .map((path) => projects.find((p) => p.path.toString() === path))
        .filter(Boolean) as ProjectItem[];
      if (found.length === 0) return;
      setOpenProjects(found);
      const active = found.find((p) => p.path.toString() === saved.active);
      if (active) openProject(active);
    } catch {
      // Corrupt saved state is not worth surfacing — start with no tabs.
    }
  }, [loading, projects]);

  // Persist the open-tab set (paths only — sessions don't survive restarts).
  useEffect(() => {
    if (!tabsRestoredRef.current) return;
    try {
      localStorage.setItem(
        "projm.openTabs",
        JSON.stringify({
          paths: openProjects.map((p) => p.path.toString()),
          active: selectedProject?.path.toString() ?? null,
        })
      );
    } catch {
      // Quota/privacy-mode failures only cost tab restore on next launch.
    }
  }, [openProjects, selectedProject]);

  useHotkey("Escape", () => {
    setSearchOpen(false);
  });

  useHotkey("Mod+Comma" as any, (e: KeyboardEvent) => {
    e.preventDefault();
    setOverlay({ kind: "settings" });
  });

  // Run Environment Diagnostics Check
  async function handleDiagnostics() {
    setRunningDiagnostics(true);
    setDiagnosticText("Running environment diagnostics...\n");
    setActiveTab("diagnostics");
    setSelectedProject(null);

    try {
      // Simulate/trigger env diagnostics
      await invoke("cmd_check_environment");
      setDiagnosticText((prev) => prev + "✓ Scan complete. Environment is clean!\n");
    } catch (err) {
      setDiagnosticText((prev) => prev + `✗ Error: ${err}\n`);
    } finally {
      setRunningDiagnostics(false);
    }
  }

  async function handleScan() {
    if (!config?.base) return;
    setScanning(true);
    try {
      await invoke("cmd_scan_directory", { path: config.base, dryRun: false });
      const prjs = await invoke<ProjectItem[]>("cmd_list_projects");
      setProjects(prjs);
    } catch (err) {
      console.error("Scan trigger failed", err);
    } finally {
      setScanning(false);
    }
  }

  // File tree node expansion toggler
  async function handleToggleExpand(nodePath: string) {
    const nextExpanded = new Set(expandedPaths);
    if (nextExpanded.has(nodePath)) {
      nextExpanded.delete(nodePath);
      setExpandedPaths(nextExpanded);
    } else {
      nextExpanded.add(nodePath);
      setExpandedPaths(nextExpanded);
      
      if (!loadedChildren[nodePath]) {
        try {
          const children = await invoke<FileEntry[]>("cmd_read_dir", { path: nodePath });
          setLoadedChildren(prev => ({
            ...prev,
            [nodePath]: children
          }));
        } catch (err) {
          console.error("Failed to load directory children", err);
        }
      }
    }
  }

  // Auto-load root folder children when a project is selected
  useEffect(() => {
    if (selectedProject) {
      const rootPath = selectedProject.path.toString();
      async function loadRoot() {
        try {
          const children = await invoke<FileEntry[]>("cmd_read_dir", { path: rootPath });
          setLoadedChildren(prev => ({
            ...prev,
            [rootPath]: children
          }));
          setExpandedPaths(new Set([rootPath]));
        } catch (err) {
          console.error("Failed to load root project folder", err);
        }
      }
      loadRoot();
    }
  }, [selectedProject]);

  // Filter projects by category and search query
  const filteredProjects = projects.filter((p) => {
    const matchCat = p.category.toLowerCase() === selectedCategory.toLowerCase();
    const matchSearch = p.name.toLowerCase().includes(searchTerm.toLowerCase());
    return matchCat && matchSearch;
  });

  return (
    <TooltipProvider>
      <div className="relative w-full h-full flex bg-background text-foreground font-sans select-none">
        
        {/* ── SIDEBAR 1: Left Narrow Icon Column (48px) ── */}
        <div className="w-12 h-full flex flex-col justify-between items-center py-4 bg-background border-r border-border/30">
          <div className="flex flex-col gap-4 items-center w-full">
            
            {/* Top workspace logo/dashboard shortcut */}
            <Tooltip>
              <TooltipTrigger>
                <button 
                  onClick={() => { setSelectedProject(null); setActiveTab("projects"); }}
                  className="w-8 h-8 rounded-lg flex items-center justify-center bg-card border border-border/20 text-primary hover:text-primary hover:bg-muted transition-all"
                >
                  <Sparkles className="size-4" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">
                <span className="font-medium text-foreground">Overview Dashboard</span>
              </TooltipContent>
            </Tooltip>
            
            <div className="w-6 h-px bg-border/20 my-1" />

            {/* Category Avatars */}
            {categories.map((cat, idx) => {
              const isSelected = selectedCategory === cat.id && activeTab === "projects" && selectedProject === null;
              const count = projects.filter((p) => p.category.toLowerCase() === cat.id.toLowerCase()).length;
              const meta = getCategoryMeta(cat.id, idx);
              return (
                <Tooltip key={cat.id}>
                  <TooltipTrigger>
                    <button
                      onClick={() => {
                        setSelectedCategory(cat.id);
                        setSelectedProject(null);
                        setActiveTab("projects");
                      }}
                      className={`w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold transition-all relative border ${
                        isSelected
                          ? `${meta.bg} ${meta.border} ${meta.color} shadow-md scale-105`
                          : "bg-card border-transparent text-muted-foreground hover:text-foreground hover:bg-muted"
                      }`}
                    >
                      {meta.letter}
                      {/* Active indicator dot */}
                      {isSelected && (
                        <span className={`absolute left-0 top-2.5 w-1 h-3 rounded-r-md ${meta.progressColor}`} />
                      )}
                    </button>
                  </TooltipTrigger>
                  <TooltipContent side="right">
                    <div className="flex flex-col gap-0.5">
                      <span className="font-semibold text-foreground">{meta.name}</span>
                      <span className="text-[10px] text-muted-foreground font-mono">{count} projects</span>
                    </div>
                  </TooltipContent>
                </Tooltip>
              );
            })}
          </div>

          {/* Bottom Gear & Help Icons */}
          <div className="flex flex-col gap-3 items-center w-full">
            <Tooltip>
              <TooltipTrigger>
                <button
                  onClick={() => setShowAddProject(true)}
                  className="text-muted-foreground hover:text-foreground transition-colors"
                >
                  <FolderPlus className="size-4" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">
                <span className="font-medium text-foreground">Add Project</span>
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger>
                <button
                  onClick={loadData}
                  className="text-muted-foreground hover:text-foreground transition-colors"
                >
                  <RefreshCw className="size-4 animate-hover" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">
                <span className="font-medium text-foreground">Reload Workspace</span>
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger>
                <button
                  type="button"
                  onClick={() => setOverlay({ kind: "settings" })}
                  className="text-muted-foreground hover:text-foreground transition-colors animate-none"
                >
                  <Settings className="size-4" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">
                <span className="font-medium text-foreground">Settings</span>
              </TooltipContent>
            </Tooltip>
          </div>
        </div>

        {/* ── SIDEBAR 2: Directory Folder Tree Column (240px) ── */}
        <div className="w-60 h-full flex flex-col justify-between bg-card border-r border-border/30">
          
          {/* Sidebar Header */}
          <div className="p-3 border-b border-border/20 flex flex-col gap-2">
            <div className="flex items-center justify-between">
              <span className="text-xs font-semibold tracking-wider text-muted-foreground uppercase font-mono">
                {config?.base ? config.base.split("/").pop() : "Workspace"}
              </span>
              <div className="flex items-center gap-1.5 text-muted-foreground">
                <button title="Add Project" onClick={() => setShowAddProject(true)}>
                  <FolderPlus className="size-3.5 hover:text-foreground cursor-pointer" />
                </button>
                <span title="New File"><FilePlus className="size-3.5 hover:text-foreground cursor-pointer" /></span>
              </div>
            </div>
            
            {/* Quick Search */}
            <div className="relative flex items-center mt-1">
              <Search className="absolute left-2 size-3 text-muted-foreground" />
              <input
                type="text"
                placeholder="Search projects..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="w-full pl-7 pr-2 py-1 text-xs bg-card border border-border/10 rounded-md focus:outline-none focus:border-border/30 focus:bg-muted"
              />
            </div>
          </div>

          {selectedProject ? (
            /* Explorer View when a project is selected (VS Code Explorer layout) */
            <div className="flex-1 overflow-y-auto scrollbar-none flex flex-col">
              <CollapsiblePanel
                title={selectedProject.name.toLowerCase()}
                isExpanded={explorerExpanded}
                onToggle={() => setExplorerExpanded(!explorerExpanded)}
                paddingClass="p-0"
              >
                <div className="flex flex-col w-full mb-1">
                  <FileTreeNode
                    entry={{
                      name: selectedProject.name.toString(),
                      path: selectedProject.path.toString(),
                      is_dir: true,
                      git_status: null
                    }}
                    depth={0}
                    expandedPaths={expandedPaths}
                    loadedChildren={loadedChildren}
                    onToggleExpand={handleToggleExpand}
                  />
                </div>
              </CollapsiblePanel>

              <CollapsiblePanel
                title="Outline"
                isExpanded={outlineExpanded}
                onToggle={() => setOutlineExpanded(!outlineExpanded)}
                paddingClass="px-4 py-2"
              >
                <div className="text-[10px] text-muted-foreground font-sans leading-relaxed">
                  The active editor cannot provide outline information.
                </div>
              </CollapsiblePanel>

              <CollapsiblePanel
                title="Timeline"
                isExpanded={timelineExpanded}
                onToggle={() => setTimelineExpanded(!timelineExpanded)}
                paddingClass="px-4 py-2"
              >
                <div className="text-[10px] text-muted-foreground font-sans leading-relaxed">
                  No timeline information available.
                </div>
              </CollapsiblePanel>

              <CollapsiblePanel
                title="File Tree Pro"
                isExpanded={fileTreeProExpanded}
                onToggle={() => setFileTreeProExpanded(!fileTreeProExpanded)}
                paddingClass="px-4 py-2"
              >
                <div className="text-[10px] text-muted-foreground font-sans leading-relaxed">
                  Advanced project metrics fully synchronized.
                </div>
              </CollapsiblePanel>
            </div>
          ) : (
            /* Categories & Projects Lists when no project is selected */
            <div className="flex-1 overflow-y-auto scrollbar-none flex flex-col">
              <CollapsiblePanel
                title="Categories"
                isExpanded={categoriesExpanded}
                onToggle={() => setCategoriesExpanded(!categoriesExpanded)}
                paddingClass="px-2 py-2"
              >
                <div className="flex flex-col gap-0.5">
                  {categories.map((cat, idx) => {
                    const count = projects.filter(
                      (p) => p.category.toLowerCase() === cat.id.toLowerCase()
                    ).length;
                    const isSelected = selectedCategory === cat.id && activeTab === "projects" && selectedProject === null;
                    const meta = getCategoryMeta(cat.id, idx);
                    return (
                      <button
                        key={cat.id}
                        onClick={() => {
                          setSelectedCategory(cat.id);
                          setSelectedProject(null);
                          setActiveTab("projects");
                        }}
                        className={`w-full flex items-center justify-between px-2 py-1 rounded-md text-[11px] font-mono group transition-all ${
                          isSelected
                            ? "bg-card text-foreground"
                            : "text-muted-foreground hover:bg-card/60 hover:text-foreground"
                        }`}
                      >
                        <div className="flex items-center gap-2 truncate">
                          <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${meta.progressColor}`} />
                          <span className="truncate">{cat.name}</span>
                        </div>
                        {count > 0 && (
                          <span className={`text-[9px] px-1.5 py-0.2 rounded font-mono font-bold ${meta.color} bg-background/40 border border-border`}>
                            {count}
                          </span>
                        )}
                      </button>
                    );
                  })}
                </div>
              </CollapsiblePanel>

              <CollapsiblePanel
                title="Projects"
                isExpanded={projectsExpanded}
                onToggle={() => setProjectsExpanded(!projectsExpanded)}
                paddingClass="px-2 py-2"
              >
                <div className="flex flex-col gap-0.5">
                  {loading ? (
                    <div className="flex items-center justify-center h-20 text-xs text-muted-foreground">
                      Loading projects...
                    </div>
                  ) : filteredProjects.length === 0 ? (
                    <div className="text-center py-6 text-xs text-muted-foreground flex flex-col items-center gap-2">
                      <FolderOpen className="size-6 text-muted-foreground stroke-1" />
                      <span>No projects here</span>
                    </div>
                  ) : (
                    filteredProjects.map((p) => {
                      const isSelected = false;
                      return (
                        <button
                          key={p.path.toString()}
                          onClick={() => openProject(p)}
                          className={`w-full flex items-center justify-between px-2 py-1.5 rounded-md text-xs font-mono group transition-all ${
                            isSelected
                              ? "bg-card border-l-2 border-primary text-primary"
                              : "text-muted-foreground hover:bg-card/60 hover:text-foreground"
                          }`}
                        >
                          <div className="flex items-center gap-2 truncate">
                            <FolderTree className={`size-3.5 shrink-0 ${isSelected ? "text-primary" : "text-muted-foreground group-hover:text-muted-foreground"}`} />
                            <span className="truncate">{p.name}</span>
                          </div>

                          <div className="flex items-center gap-1 shrink-0">
                            {/* Set category (hover) */}
                            <span
                              role="button"
                              tabIndex={-1}
                              title="Set category…"
                              onClick={(e) => {
                                e.stopPropagation();
                                openAssignDialog(p);
                              }}
                              className="opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-accent text-muted-foreground hover:text-primary transition-all"
                            >
                              <Tag className="size-3" />
                            </span>

                            {/* Git branch & Dirty indicators */}
                            {p.git_branch && (
                              <div className="flex items-center gap-1 shrink-0 bg-card px-1 rounded-sm border border-border/10">
                                <GitBranch className="size-2.5 text-muted-foreground" />
                                <span className="text-[10px] text-muted-foreground font-sans truncate max-w-[50px]">
                                  {p.git_branch}
                                </span>
                                <span
                                  className={`w-1 h-1 rounded-full ${
                                    p.git_dirty ? "bg-amber-400" : "bg-emerald-400"
                                  }`}
                                  title={p.git_dirty ? "Dirty changes" : "Clean repository"}
                                />
                              </div>
                            )}
                          </div>
                        </button>
                      );
                    })
                  )}
                </div>
               </CollapsiblePanel>
            </div>
          )}

          {/* Sidebar Switched Footer */}
          <div className="p-2 bg-background border-t border-border/30 flex flex-col gap-1">
            <button
              onClick={() => { setSelectedProject(null); setActiveTab("projects"); }}
              className={`w-full flex items-center gap-2 px-3 py-1.5 rounded text-xs transition-colors ${
                selectedProject === null && activeTab === "projects"
                  ? "bg-card text-primary"
                  : "text-muted-foreground hover:bg-card hover:text-foreground"
              }`}
            >
              <FolderTree className="size-3.5" />
              <span>Files View</span>
            </button>
            
            <button
              onClick={handleDiagnostics}
              disabled={runningDiagnostics}
              className={`w-full flex items-center gap-2 px-3 py-1.5 rounded text-xs transition-colors ${
                activeTab === "diagnostics"
                  ? "bg-card text-primary"
                  : "text-muted-foreground hover:bg-card hover:text-foreground"
              }`}
            >
              <HeartPulse className="size-3.5" />
              <span>{runningDiagnostics ? "Running Diagnostics..." : "Diagnostics"}</span>
            </button>
          </div>
        </div>

        {/* ── MAIN WORKSPACE VIEWPORT (Remaining space) ── */}
        <div className="flex-1 h-full flex flex-col bg-background">
          
          {/* Workspace Top Header Panel */}
          <div className="h-10 border-b border-border/30 flex items-center justify-between px-4 bg-background">
            
            {/* Left Arrow Controls and Open-Project Tabs */}
            <div className="flex items-center gap-3 flex-1 min-w-0">
              <div className="flex items-center gap-1 shrink-0">
                <button
                  onClick={() => setSelectedProject(null)}
                  className="p-1 rounded hover:bg-card text-muted-foreground hover:text-foreground"
                  title="Go Back"
                >
                  <ChevronRight className="size-3.5 rotate-180" />
                </button>
                <button className="p-1 rounded hover:bg-card text-muted-foreground hover:text-foreground disabled:opacity-30" disabled>
                  <ChevronRight className="size-3.5" />
                </button>
              </div>
              {openProjects.length > 0 ? (
                <ProjectTabs
                  projects={openProjects}
                  activePath={selectedProject?.path.toString() ?? null}
                  activity={runnerActivity}
                  onSelect={(p) => openProject(p as ProjectItem)}
                  onClose={closeProject}
                />
              ) : (
                <span className="text-xs text-muted-foreground font-mono">
                  {activeTab === "diagnostics"
                    ? "Environment Diagnostics"
                    : "Overview Dashboard"}
                </span>
              )}
            </div>

            {/* Right Header Operations */}
            <div className="flex items-center gap-2">
              <button 
                onClick={handleScan}
                disabled={scanning}
                className="flex items-center gap-1.5 bg-primary hover:bg-primary/90 text-foreground font-medium text-xs px-3 py-1.5 rounded transition-all active:scale-95 disabled:opacity-50"
                title="Scan base workspaces directory"
              >
                <RefreshCw className={`size-3 text-primary ${scanning ? "animate-spin" : ""}`} />
                <span>{scanning ? "Scanning..." : "Scan Workspace"}</span>
              </button>
              <button
                type="button"
                onClick={() => setOverlay({ kind: "settings" })}
                className="p-1.5 rounded hover:bg-card text-muted-foreground hover:text-foreground transition-colors"
                title="Settings"
              >
                <Settings2 className="size-3.5" />
              </button>
            </div>
          </div>

          {/* ── Workspace Body Content ── */}
          <div className="flex-1 p-4 overflow-hidden flex flex-col">
            
            {/* CASE 1: Per-tab Run & Shell panels. Every activated open project
                keeps its RunnerPanel mounted (hidden when inactive) so shells,
                logs, and runner sessions survive tab switches. */}
            <div
              className={
                selectedProject
                  ? "flex-1 flex flex-col overflow-hidden min-h-0"
                  : "hidden"
              }
            >
              {selectedProject && (
                <div className="mb-2 flex items-center justify-between shrink-0">
                  <div className="flex items-center gap-2">
                    <TerminalIcon className="size-4 text-primary" />
                    <span className="text-sm font-semibold tracking-wide font-mono text-primary">
                      {selectedProject.name} — Run &amp; Shell
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-[10px] bg-muted border border-border/20 px-2 py-0.5 rounded font-mono text-emerald-400">
                      DEV RUNNER
                    </span>
                  </div>
                </div>
              )}
              {openProjects
                .filter((p) => activatedPaths.has(p.path.toString()))
                .map((p) => {
                  const path = p.path.toString();
                  const isActivePanel =
                    selectedProject?.path.toString() === path;
                  return (
                    <div
                      key={path}
                      className={
                        isActivePanel
                          ? "flex-1 flex flex-col min-h-0"
                          : "hidden"
                      }
                    >
                      <RunnerPanel
                        project={{ name: p.name.toString(), path }}
                        onOpenSettings={(tab) =>
                          setOverlay({ kind: "settings", tab })
                        }
                      />
                    </div>
                  );
                })}
            </div>
            {selectedProject ? null : activeTab === "diagnostics" ? (
              /* CASE 2: Environment Diagnostics Console View */
              <div className="flex-1 flex flex-col overflow-hidden bg-background border border-border rounded-lg p-4 font-mono text-sm">
                <div className="flex items-center justify-between border-b border-border/20 pb-2 mb-3">
                  <div className="flex items-center gap-2 text-primary">
                    <HeartPulse className="size-4 text-pink-400" />
                    <span>Projm Environment Scan Logs</span>
                  </div>
                  <button
                    onClick={handleDiagnostics}
                    disabled={runningDiagnostics}
                    className="bg-primary hover:bg-primary/90 text-foreground font-sans text-xs px-3 py-1 rounded transition-colors disabled:opacity-50"
                  >
                    {runningDiagnostics ? "Scanning..." : "Re-run Scan"}
                  </button>
                </div>
                <pre className="flex-1 overflow-y-auto whitespace-pre-wrap text-emerald-400 scrollbar-thin">
                  {diagnosticText}
                </pre>
              </div>
            ) : (
              /* CASE 3: Clean Dashboard Overview */
              <div className="flex-1 overflow-y-auto flex flex-col gap-4 max-w-5xl mx-auto w-full py-2 scrollbar-thin">
                
                {/* Header section with inline Base Location */}
                <div className="flex items-center justify-between border-b border-border pb-3">
                  <div className="flex flex-col gap-0.5">
                    <h1 className="text-xl font-bold tracking-tight bg-gradient-to-r from-foreground via-foreground to-muted-foreground bg-clip-text text-transparent">
                      Workspace Dashboard
                    </h1>
                    <p className="text-[10px] text-muted-foreground">Overview of organized workspace directories and Git configurations.</p>
                  </div>
                  
                  <div className="flex items-center gap-1.5 text-[10px] bg-card border border-border rounded px-2 py-1 font-mono text-muted-foreground">
                    <span className="text-muted-foreground">Base path:</span>
                    <span className="text-primary truncate max-w-[140px] sm:max-w-xs">{config?.base ?? "Not configured"}</span>
                    <button type="button" onClick={() => setOverlay({ kind: "settings" })} className="text-primary hover:text-primary ml-1 font-sans transition-colors font-medium">
                      Change →
                    </button>
                  </div>
                </div>

                {/* Global Workspace Health Stats Row */}
                <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-3">
                  {/* Card 1: Total projects */}
                  <div className="p-3 rounded-lg border border-border bg-card flex items-center justify-between group hover:border-border/50 transition-all">
                    <div className="flex flex-col gap-0.5">
                      <span className="text-[9px] tracking-wider text-muted-foreground font-mono uppercase">
                        Total Projects
                      </span>
                      <span className="text-lg font-bold text-foreground font-mono">
                        {projects.length}
                      </span>
                      <span className="text-[9px] text-muted-foreground">Classified items</span>
                    </div>
                    <div className="p-1.5 rounded bg-primary/10 text-primary">
                      <FolderTree className="size-4" />
                    </div>
                  </div>

                  {/* Card 2: Active categories */}
                  {(() => {
                    const activeCats = new Set(projects.map(p => p.category.toLowerCase())).size;
                    return (
                      <div className="p-3 rounded-lg border border-border bg-card flex items-center justify-between group hover:border-border/50 transition-all">
                        <div className="flex flex-col gap-0.5">
                          <span className="text-[9px] tracking-wider text-muted-foreground font-mono uppercase">
                            Active Folders
                          </span>
                          <span className="text-lg font-bold text-emerald-400 font-mono">
                            {activeCats} <span className="text-xs text-muted-foreground font-normal">/ {categories.length}</span>
                          </span>
                          <span className="text-[9px] text-muted-foreground">Populated folders</span>
                        </div>
                        <div className="p-1.5 rounded bg-emerald-500/10 text-emerald-400">
                          <Layers className="size-4" />
                        </div>
                      </div>
                    );
                  })()}

                  {/* Card 3: Last scanned project */}
                  {(() => {
                    const lastProject = projects[0];
                    const meta = lastProject ? getCategoryMeta(lastProject.category, 0) : null;
                    return (
                      <div className="p-3 rounded-lg border border-border bg-card flex items-center justify-between group hover:border-border/50 transition-all">
                        <div className="flex flex-col gap-0.5 truncate w-full">
                          <span className="text-[9px] tracking-wider text-muted-foreground font-mono uppercase">
                            Last Scanned
                          </span>
                          <span className="text-xs font-semibold text-primary font-mono truncate max-w-[130px]" title={lastProject?.name.toString() ?? "None"}>
                            {lastProject?.name ?? "None"}
                          </span>
                          {lastProject ? (
                            <span className={`text-[8px] px-1 py-0.2 rounded font-mono font-bold capitalize border w-fit ${meta?.color} ${meta?.bg} ${meta?.border}`}>
                              {lastProject.category}
                            </span>
                          ) : (
                            <span className="text-[9px] text-muted-foreground">No scans executed</span>
                          )}
                        </div>
                        <div className="p-1.5 rounded bg-primary/10 text-primary">
                          <Activity className="size-4" />
                        </div>
                      </div>
                    );
                  })()}

                  {/* Card 4: Untracked projects */}
                  {(() => {
                    const untrackedCount = projects.filter(p => !p.git_branch).length;
                    return (
                      <div className="p-3 rounded-lg border border-border bg-card flex items-center justify-between group hover:border-border/50 transition-all">
                        <div className="flex flex-col gap-0.5">
                          <span className="text-[9px] tracking-wider text-muted-foreground font-mono uppercase">
                            Untracked
                          </span>
                          <span className={`text-lg font-bold font-mono ${untrackedCount > 0 ? "text-amber-400" : "text-muted-foreground"}`}>
                            {untrackedCount}
                          </span>
                          <span className="text-[9px] text-muted-foreground">Missing Git tracking</span>
                        </div>
                        <div className={`p-1.5 rounded bg-amber-500/10 text-amber-400 ${untrackedCount > 0 ? "" : "bg-muted text-muted-foreground"}`}>
                          <GitBranch className="size-4" />
                        </div>
                      </div>
                    );
                  })()}
                </div>

                {/* Categories Grid List */}
                <div className="flex flex-col gap-2">
                  <span className="text-[10px] font-mono text-muted-foreground tracking-wider uppercase">
                    Per-Category Project Breakdown
                  </span>
                  
                  <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-3">
                    {categories.map((cat, idx) => {
                      const count = projects.filter((p) => p.category.toLowerCase() === cat.id.toLowerCase()).length;
                      const percentage = projects.length > 0 ? (count / projects.length) * 100 : 0;
                      const meta = getCategoryMeta(cat.id, idx);
                      const isPopulated = count > 0;

                      return (
                        <button
                          key={cat.id}
                          onClick={() => {
                            setSelectedCategory(cat.id);
                            setSelectedProject(null);
                            setActiveTab("projects");
                          }}
                          className={`p-3 rounded-lg border text-left flex flex-col justify-between group relative overflow-hidden transition-all duration-300 min-h-[92px] ${
                            isPopulated
                              ? `bg-card hover:bg-card border-border hover:border-${meta.accentHex}/30 shadow-[0_4px_20px_rgba(0,0,0,0.4)]`
                              : "bg-background/40 border-border opacity-55 hover:opacity-80"
                          }`}
                          style={isPopulated ? {
                            boxShadow: `inset 0 1px 0 0 rgba(255,255,255,0.03), 0 4px 20px rgba(0,0,0,0.4)`
                          } : undefined}
                        >
                          <div className="flex items-start justify-between w-full">
                            <div className="flex items-center gap-2">
                              <span className={`w-4 h-4 rounded flex items-center justify-center text-[9px] font-mono font-bold bg-background/40 border border-border ${isPopulated ? meta.color : "text-muted-foreground"}`}>
                                {meta.letter}
                              </span>
                              <span className={`text-[11px] font-semibold capitalize tracking-wide ${isPopulated ? "text-foreground group-hover:text-foreground" : "text-muted-foreground"}`}>
                                {cat.name}
                              </span>
                            </div>
                            
                            <span className={`text-[10px] font-mono px-1.5 py-0.2 rounded ${isPopulated ? `${meta.color} bg-background/40 border border-border` : "text-muted-foreground bg-background/10"}`}>
                              {count}
                            </span>
                          </div>

                          <div className="w-full mt-3 flex flex-col gap-1">
                            <span className={`text-[9px] ${isPopulated ? "text-muted-foreground font-medium" : "text-muted-foreground"}`}>
                              {isPopulated ? `${count} ${count === 1 ? "project" : "projects"}` : "empty folder"}
                            </span>
                            
                            {/* Proportional Fill Progress Bar */}
                            <div className="w-full h-1 rounded-full bg-muted border border-border overflow-hidden">
                              {isPopulated && (
                                <div 
                                  className={`h-full rounded-full transition-all duration-500 ${meta.progressColor}`}
                                  style={{ width: `${percentage}%` }}
                                />
                              )}
                            </div>
                          </div>
                        </button>
                      );
                    })}
                  </div>
                </div>

                {/* Recent Workspace Activity / Projects Row */}
                <div className="p-3 rounded-lg border border-border bg-card flex flex-col gap-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-1.5">
                      <Activity className="size-3.5 text-primary" />
                      <span className="text-xs font-semibold tracking-wide text-foreground">
                        Recent Workspace Activity
                      </span>
                    </div>
                    <span className="text-[9px] text-muted-foreground font-mono">
                      Updated just now
                    </span>
                  </div>
                  
                  {projects.length === 0 ? (
                    <div className="text-center py-4 text-xs text-muted-foreground italic">
                      No projects scanned yet. Click "Scan Workspace" in the top-bar to populate.
                    </div>
                  ) : (
                    <div className="flex flex-col gap-1">
                      {projects.slice(0, 5).map((p) => {
                        const meta = getCategoryMeta(p.category, 0);
                        const tags = getStackTags(p);
                        return (
                          <button
                            key={p.path.toString()}
                            onClick={() => {
                              openProject(p);
                              setSelectedCategory(p.category);
                            }}
                            className="w-full flex flex-col sm:flex-row sm:items-center sm:justify-between p-2 rounded border border-border bg-background/20 hover:bg-card/60 transition-all text-left group gap-2"
                          >
                            <div className="flex items-center gap-2 truncate">
                              <FolderTree className="size-3.5 text-muted-foreground group-hover:text-primary shrink-0 transition-colors" />
                              <div className="flex flex-col gap-0.2 truncate">
                                <span className="text-xs font-mono font-semibold text-foreground group-hover:text-foreground truncate">
                                  {p.name}
                                </span>
                                <span className="text-[9px] text-muted-foreground font-mono truncate max-w-[200px] sm:max-w-xs">
                                  {p.path}
                                </span>
                              </div>
                            </div>
                            
                            <div className="flex items-center gap-2 sm:gap-3 shrink-0 flex-wrap sm:flex-nowrap">
                              {/* Stack Tags */}
                              <div className="flex gap-0.5">
                                {tags.map((tag) => (
                                  <span 
                                    key={tag} 
                                    className="text-[8px] px-1 py-0.2 rounded bg-muted/40 text-muted-foreground border border-border font-sans"
                                  >
                                    {tag}
                                  </span>
                                ))}
                              </div>
                              
                              {/* Category Badge — click to reassign */}
                              <span
                                role="button"
                                tabIndex={0}
                                title="Set category…"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  openAssignDialog(p);
                                }}
                                onKeyDown={(e) => {
                                  if (e.key === "Enter" || e.key === " ") {
                                    e.stopPropagation();
                                    openAssignDialog(p);
                                  }
                                }}
                                className={`text-[8px] px-1.5 py-0.2 rounded font-mono font-bold capitalize border cursor-pointer hover:ring-1 hover:ring-primary/40 ${meta.color} ${meta.bg} ${meta.border}`}
                              >
                                {p.category}
                              </span>
                              
                              {/* Branch status */}
                              {p.git_branch ? (
                                <div className="flex items-center gap-1 bg-background/40 px-1.5 py-0.2 rounded border border-border text-[9px] text-muted-foreground font-mono">
                                  <GitBranch className="size-2.5 text-muted-foreground" />
                                  <span className="truncate max-w-[70px]">{p.git_branch}</span>
                                  <span className={`w-1 h-1 rounded-full ${p.git_dirty ? "bg-amber-400 animate-pulse" : "bg-emerald-400"}`} />
                                </div>
                              ) : (
                                <span className="text-[8px] text-muted-foreground font-mono">untracked</span>
                              )}
                              
                              <ChevronRight className="size-3 text-muted-foreground group-hover:text-foreground group-hover:translate-x-0.5 transition-all hidden sm:block" />
                            </div>
                          </button>
                        );
                      })}
                    </div>
                  )}
                </div>

                {/* Scan Tips / Helpful Banner */}
                <div className="p-3 rounded-lg border border-border bg-card flex gap-2.5 items-start">
                  <Info className="size-4 text-primary shrink-0 mt-0.5" />
                  <div className="flex flex-col gap-1 text-[11px] w-full sm:flex-row sm:justify-between sm:items-center">
                    <div className="flex flex-col gap-0.2">
                      <span className="font-semibold text-foreground">How to populate the directories?</span>
                      <span className="text-muted-foreground leading-relaxed max-w-xl">
                        Enter the path to your source directories using the Scan page. The Projm background compiler automatically parses stacks, categorizes languages, and sets up Git tracking triggers.
                      </span>
                    </div>
                    <button
                      type="button"
                      onClick={() => setOverlay({ kind: "scan" })}
                      className="flex items-center gap-0.5 text-[10px] text-primary font-medium hover:text-primary shrink-0 mt-1 sm:mt-0 bg-primary/10 px-2.5 py-1 rounded border border-primary/20"
                    >
                      <span>Scan settings</span>
                      <ChevronRight className="size-2.5" />
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* ── Settings / Scan overlays ── rendered on top of the workspace so
            RunnerPanels stay mounted and shell sessions survive. Scoped to this
            (relative) container, so the titlebar stays visible above. ── */}
        {overlay?.kind === "settings" && (
          <div className="absolute inset-0 z-40 bg-background overflow-hidden flex flex-col">
            <SettingsPanel
              onClose={() => setOverlay(null)}
              initialTab={overlay.tab}
            />
          </div>
        )}
        {overlay?.kind === "scan" && (
          <div className="absolute inset-0 z-40 bg-background overflow-y-auto">
            <div className="p-6 lg:p-8 max-w-3xl mx-auto">
              <ScanPanel onClose={() => setOverlay(null)} />
            </div>
          </div>
        )}

        {/* ── Add Project Dialog ── */}
        {showAddProject && (
          <AddProjectDialog
            categories={categories}
            base={config?.base}
            onClose={() => setShowAddProject(false)}
            onAdded={() => {
              setShowAddProject(false);
              loadData();
            }}
          />
        )}

        {/* ── Assign Category Dialog ── */}
        {assignTarget && (
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/60 backdrop-blur-sm">
            <div className="w-full max-w-sm bg-card border border-border rounded-xl shadow-2xl overflow-hidden flex flex-col animate-in fade-in zoom-in-95 duration-150">
              <div className="p-4 border-b border-border/20 flex items-center gap-2">
                <Tag className="size-4 text-primary" />
                <div className="flex flex-col">
                  <span className="text-sm font-semibold text-foreground">Set category</span>
                  <span className="text-[10px] font-mono text-muted-foreground truncate max-w-[260px]">
                    {assignTarget.name}
                  </span>
                </div>
              </div>

              <div className="p-4 flex flex-col gap-4">
                {/* Category picker */}
                <div className="space-y-1.5">
                  <label className="text-[9px] font-bold text-muted-foreground uppercase tracking-widest">
                    Category
                  </label>
                  <div className="flex flex-wrap gap-1.5">
                    {categories.map((cat) => (
                      <button
                        key={cat.id}
                        onClick={() => setAssignCategory(cat.id)}
                        className={`px-2 py-1 rounded-md text-[10px] font-mono capitalize border transition-all ${
                          assignCategory === cat.id
                            ? "bg-primary/15 text-primary border-primary/40"
                            : "bg-muted/20 text-muted-foreground border-border hover:text-foreground"
                        }`}
                      >
                        {cat.id}
                      </button>
                    ))}
                  </div>
                </div>

                {/* Persist method */}
                <div className="space-y-1.5">
                  <label className="text-[9px] font-bold text-muted-foreground uppercase tracking-widest">
                    Save as
                  </label>
                  <div className="flex flex-col gap-1.5">
                    <label className="flex items-start gap-2 text-xs cursor-pointer">
                      <input
                        type="radio"
                        checked={assignMode === "marker"}
                        onChange={() => setAssignMode("marker")}
                        className="mt-0.5 accent-[var(--primary)]"
                      />
                      <span className="flex flex-col">
                        <span className="text-foreground">Pin with .projm.toml</span>
                        <span className="text-[10px] text-muted-foreground">
                          Travels with the repo; overrides all rules
                        </span>
                      </span>
                    </label>
                    <label className="flex items-start gap-2 text-xs cursor-pointer">
                      <input
                        type="radio"
                        checked={assignMode === "rule"}
                        onChange={() => setAssignMode("rule")}
                        className="mt-0.5 accent-[var(--primary)]"
                      />
                      <span className="flex flex-col">
                        <span className="text-foreground">Add global exact-name rule</span>
                        <span className="text-[10px] text-muted-foreground">
                          Stored in rules.toml, evaluated first
                        </span>
                      </span>
                    </label>
                  </div>
                </div>

                <label className="flex items-center gap-2 text-xs cursor-pointer">
                  <input
                    type="checkbox"
                    checked={assignMove}
                    onChange={(e) => setAssignMove(e.target.checked)}
                    className="accent-[var(--primary)]"
                  />
                  <span className="text-foreground">Move project folder now</span>
                </label>

                {assignError && (
                  <p className="text-[10px] text-rose-400 font-mono break-all">{assignError}</p>
                )}
              </div>

              <div className="p-3 border-t border-border/20 flex justify-end gap-2">
                <button
                  onClick={() => setAssignTarget(null)}
                  className="text-xs px-3 py-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={handleAssignCategory}
                  disabled={assignBusy || !assignCategory}
                  className="text-xs px-3 py-1.5 rounded-md bg-primary hover:bg-primary/90 text-foreground font-semibold disabled:opacity-50 transition-all"
                >
                  {assignBusy ? "Applying…" : "Apply"}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* ── Quick Ctrl+K Modal Command Finder ── */}
        {searchOpen && (
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/60 backdrop-blur-sm">
            <div className="w-full max-w-md bg-card border border-border rounded-xl shadow-2xl overflow-hidden flex flex-col animate-in fade-in zoom-in-95 duration-150">
              <div className="p-3 border-b border-border/20 flex items-center gap-2">
                <Search className="size-4 text-muted-foreground" />
                <input
                  type="text"
                  placeholder="Find a project by name..."
                  autoFocus
                  onChange={(e) => setSearchTerm(e.target.value)}
                  className="w-full bg-transparent focus:outline-none text-sm text-foreground"
                />
                <button 
                  onClick={() => setSearchOpen(false)}
                  className="text-[10px] bg-muted hover:bg-muted px-2 py-0.5 rounded text-muted-foreground font-mono"
                >
                  ESC
                </button>
              </div>
              
              <div className="max-h-60 overflow-y-auto p-2 flex flex-col gap-1">
                {projects
                  .filter((p) => p.name.toLowerCase().includes(searchTerm.toLowerCase()))
                  .map((p) => (
                    <button
                      key={p.path.toString()}
                      onClick={() => {
                        openProject(p);
                        setSelectedCategory(p.category);
                        setSearchOpen(false);
                      }}
                      className="w-full text-left p-2 rounded hover:bg-card text-xs font-mono flex justify-between items-center text-foreground hover:text-foreground"
                    >
                      <span>{p.name}</span>
                      <span className="text-[10px] uppercase text-muted-foreground bg-muted border border-border/10 px-1.5 py-0.5 rounded font-sans">
                        {p.category}
                      </span>
                    </button>
                  ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </TooltipProvider>
  );
}
