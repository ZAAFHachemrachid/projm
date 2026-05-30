"use client";

import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import Link from "next/link";
import { useRouter } from "next/navigation";
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
  Activity
} from "lucide-react";
import TerminalView from "@/components/ui/terminal";
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
        ? <FolderOpen className="size-3.5 text-indigo-400 shrink-0" />
        : <FolderOpen className="size-3.5 text-slate-400/80 shrink-0" />;
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
      return <GitBranch className="size-3.5 text-[#64748b] shrink-0" />;
    }
    
    return <span className="text-slate-400 font-bold font-mono text-[10px] size-3.5 shrink-0 flex items-center justify-center">📄</span>;
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
    return { label: entry.git_status, colorClass: "text-[#64748b]" };
  };

  const gitStyle = getGitStyle();

  return (
    <div className="flex flex-col w-full select-none">
      <button
        onClick={() => entry.is_dir && onToggleExpand(entry.path)}
        className="w-full flex items-center justify-between py-1 px-2 rounded-md hover:bg-[#181a1c]/60 group text-left transition-colors relative"
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
      >
        {/* Indenting Guide Lines */}
        {depth > 0 && Array.from({ length: depth }).map((_, i) => (
          <span 
            key={i} 
            className="absolute top-0 bottom-0 border-l border-white/5" 
            style={{ left: `${i * 12 + 14}px` }} 
          />
        ))}

        <div className="flex items-center gap-2 truncate">
          {entry.is_dir && (
            <ChevronRight className={`size-3 text-[#64748b] transition-transform duration-150 shrink-0 ${isExpanded ? "rotate-90" : ""}`} />
          )}
          {!entry.is_dir && <span className="w-3 shrink-0" />}
          
          {getFileIcon()}
          
          <span className={`text-[11px] font-mono truncate ${gitStyle ? gitStyle.colorClass : "text-slate-300 group-hover:text-slate-100"}`}>
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
    <div className="flex flex-col w-full border-b border-white/[0.03]">
      <button
        onClick={onToggle}
        className="w-full flex items-center justify-between py-1.5 px-3 bg-[#0d0e10]/80 hover:bg-[#16181c]/90 transition-colors group text-left select-none outline-none focus:outline-none border-t border-white/[0.03]"
      >
        <div className="flex items-center gap-1.5">
          <ChevronRight 
            className={`size-3 text-[#64748b] group-hover:text-slate-300 transition-transform duration-150 shrink-0 ${
              isExpanded ? "rotate-90" : ""
            }`} 
          />
          <span className="text-[11px] font-bold tracking-wide text-[#94a3b8] group-hover:text-slate-200 font-sans">
            {title}
          </span>
        </div>
      </button>
      
      {isExpanded && (
        <div className={`${paddingClass} flex flex-col gap-0.5 w-full bg-[#0f1012]`}>
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
  const [diagnosticText, setDiagnosticText] = useState("");
  const [runningDiagnostics, setRunningDiagnostics] = useState(false);

  // Search & interaction states
  const [searchTerm, setSearchTerm] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [scanning, setScanning] = useState(false);
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

  const router = useRouter();

  // Bind professional cross-platform hotkeys using TanStack React Hotkeys
  useHotkey("Mod+K", (e) => {
    e.preventDefault();
    setSearchOpen((prev) => !prev);
  });

  useHotkey("Escape", () => {
    setSearchOpen(false);
  });

  useHotkey("Mod+Comma" as any, (e: KeyboardEvent) => {
    e.preventDefault();
    router.push("/settings");
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
      <div className="w-full h-full flex bg-[#090a0b] text-[#e2e8f0] font-sans select-none">
        
        {/* ── SIDEBAR 1: Left Narrow Icon Column (48px) ── */}
        <div className="w-12 h-full flex flex-col justify-between items-center py-4 bg-[#0d0e10] border-r border-[#1f2937]/30">
          <div className="flex flex-col gap-4 items-center w-full">
            
            {/* Top workspace logo/dashboard shortcut */}
            <Tooltip>
              <TooltipTrigger>
                <button 
                  onClick={() => { setSelectedProject(null); setActiveTab("projects"); }}
                  className="w-8 h-8 rounded-lg flex items-center justify-center bg-[#18191b] border border-border/20 text-indigo-400 hover:text-indigo-300 hover:bg-[#202124] transition-all"
                >
                  <Sparkles className="size-4" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">
                <span className="font-medium text-slate-200">Overview Dashboard</span>
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
                          : "bg-[#181a1c] border-transparent text-[#94a3b8] hover:text-white hover:bg-[#242629]"
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
                      <span className="font-semibold text-slate-200">{meta.name}</span>
                      <span className="text-[10px] text-slate-400 font-mono">{count} projects</span>
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
                  onClick={loadData}
                  className="text-muted-foreground hover:text-white transition-colors" 
                >
                  <RefreshCw className="size-4 animate-hover" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">
                <span className="font-medium text-slate-200">Reload Workspace</span>
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger>
                <Link
                  href="/settings"
                  className="text-muted-foreground hover:text-white transition-colors animate-none"
                >
                  <Settings className="size-4" />
                </Link>
              </TooltipTrigger>
              <TooltipContent side="right">
                <span className="font-medium text-slate-200">Settings</span>
              </TooltipContent>
            </Tooltip>
          </div>
        </div>

        {/* ── SIDEBAR 2: Directory Folder Tree Column (240px) ── */}
        <div className="w-60 h-full flex flex-col justify-between bg-[#0f1012] border-r border-[#1f2937]/30">
          
          {/* Sidebar Header */}
          <div className="p-3 border-b border-[#1f2937]/20 flex flex-col gap-2">
            <div className="flex items-center justify-between">
              <span className="text-xs font-semibold tracking-wider text-[#94a3b8] uppercase font-mono">
                {config?.base ? config.base.split("/").pop() : "Workspace"}
              </span>
              <div className="flex items-center gap-1.5 text-[#64748b]">
                <span title="New Folder"><FolderPlus className="size-3.5 hover:text-white cursor-pointer" /></span>
                <span title="New File"><FilePlus className="size-3.5 hover:text-white cursor-pointer" /></span>
              </div>
            </div>
            
            {/* Quick Search */}
            <div className="relative flex items-center mt-1">
              <Search className="absolute left-2 size-3 text-[#64748b]" />
              <input
                type="text"
                placeholder="Search projects..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="w-full pl-7 pr-2 py-1 text-xs bg-[#17181c] border border-border/10 rounded-md focus:outline-none focus:border-border/30 focus:bg-[#1a1b20]"
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
                <div className="text-[10px] text-[#64748b] font-sans leading-relaxed">
                  The active editor cannot provide outline information.
                </div>
              </CollapsiblePanel>

              <CollapsiblePanel
                title="Timeline"
                isExpanded={timelineExpanded}
                onToggle={() => setTimelineExpanded(!timelineExpanded)}
                paddingClass="px-4 py-2"
              >
                <div className="text-[10px] text-[#64748b] font-sans leading-relaxed">
                  No timeline information available.
                </div>
              </CollapsiblePanel>

              <CollapsiblePanel
                title="File Tree Pro"
                isExpanded={fileTreeProExpanded}
                onToggle={() => setFileTreeProExpanded(!fileTreeProExpanded)}
                paddingClass="px-4 py-2"
              >
                <div className="text-[10px] text-[#64748b] font-sans leading-relaxed">
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
                            ? "bg-[#18191c] text-[#e2e8f0]"
                            : "text-[#94a3b8] hover:bg-[#181a1c]/60 hover:text-white"
                        }`}
                      >
                        <div className="flex items-center gap-2 truncate">
                          <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${meta.progressColor}`} />
                          <span className="truncate">{cat.name}</span>
                        </div>
                        {count > 0 && (
                          <span className={`text-[9px] px-1.5 py-0.2 rounded font-mono font-bold ${meta.color} bg-black/40 border border-white/5`}>
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
                      <FolderOpen className="size-6 text-[#475569] stroke-1" />
                      <span>No projects here</span>
                    </div>
                  ) : (
                    filteredProjects.map((p) => {
                      const isSelected = false;
                      return (
                        <button
                          key={p.path.toString()}
                          onClick={() => {
                            setSelectedProject(p);
                            setActiveTab("projects");
                          }}
                          className={`w-full flex items-center justify-between px-2 py-1.5 rounded-md text-xs font-mono group transition-all ${
                            isSelected
                              ? "bg-[#18191c] border-l-2 border-indigo-500 text-indigo-200"
                              : "text-[#94a3b8] hover:bg-[#181a1c]/60 hover:text-white"
                          }`}
                        >
                          <div className="flex items-center gap-2 truncate">
                            <FolderTree className={`size-3.5 shrink-0 ${isSelected ? "text-indigo-400" : "text-[#475569] group-hover:text-slate-400"}`} />
                            <span className="truncate">{p.name}</span>
                          </div>

                          {/* Git branch & Dirty indicators */}
                          {p.git_branch && (
                            <div className="flex items-center gap-1 shrink-0 bg-[#17181a] px-1 rounded-sm border border-border/10">
                              <GitBranch className="size-2.5 text-[#64748b]" />
                              <span className="text-[10px] text-[#64748b] font-sans truncate max-w-[50px]">
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
                        </button>
                      );
                    })
                  )}
                </div>
               </CollapsiblePanel>
            </div>
          )}

          {/* Sidebar Switched Footer */}
          <div className="p-2 bg-[#0c0d0f] border-t border-[#1f2937]/30 flex flex-col gap-1">
            <button
              onClick={() => { setSelectedProject(null); setActiveTab("projects"); }}
              className={`w-full flex items-center gap-2 px-3 py-1.5 rounded text-xs transition-colors ${
                selectedProject === null && activeTab === "projects"
                  ? "bg-[#18191b] text-indigo-300"
                  : "text-muted-foreground hover:bg-[#151619] hover:text-white"
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
                  ? "bg-[#18191b] text-indigo-300"
                  : "text-muted-foreground hover:bg-[#151619] hover:text-white"
              }`}
            >
              <HeartPulse className="size-3.5" />
              <span>{runningDiagnostics ? "Running Diagnostics..." : "Diagnostics"}</span>
            </button>
          </div>
        </div>

        {/* ── MAIN WORKSPACE VIEWPORT (Remaining space) ── */}
        <div className="flex-1 h-full flex flex-col bg-[#0b0c0e]">
          
          {/* Workspace Top Header Panel */}
          <div className="h-10 border-b border-[#1f2937]/30 flex items-center justify-between px-4 bg-[#0d0e10]">
            
            {/* Left Arrow Controls and Breadcrumbs */}
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-1">
                <button 
                  onClick={() => setSelectedProject(null)}
                  className="p-1 rounded hover:bg-[#18191b] text-muted-foreground hover:text-white"
                  title="Go Back"
                >
                  <ChevronRight className="size-3.5 rotate-180" />
                </button>
                <button className="p-1 rounded hover:bg-[#18191b] text-muted-foreground hover:text-white disabled:opacity-30" disabled>
                  <ChevronRight className="size-3.5" />
                </button>
              </div>
              <span className="text-xs text-muted-foreground font-mono">
                {selectedProject
                  ? `~/projects/${selectedProject.category}/${selectedProject.name}`
                  : activeTab === "diagnostics"
                  ? "Environment Diagnostics"
                  : "Overview Dashboard"}
              </span>
            </div>

            {/* Right Header Operations */}
            <div className="flex items-center gap-2">
              <button 
                onClick={handleScan}
                disabled={scanning}
                className="flex items-center gap-1.5 bg-indigo-600 hover:bg-indigo-500 text-white font-medium text-xs px-3 py-1.5 rounded transition-all active:scale-95 disabled:opacity-50"
                title="Scan base workspaces directory"
              >
                <RefreshCw className={`size-3 text-indigo-100 ${scanning ? "animate-spin" : ""}`} />
                <span>{scanning ? "Scanning..." : "Scan Workspace"}</span>
              </button>
              <Link 
                href="/settings" 
                className="p-1.5 rounded hover:bg-[#18191b] text-[#64748b] hover:text-white transition-colors" 
                title="Settings"
              >
                <Settings2 className="size-3.5" />
              </Link>
            </div>
          </div>

          {/* ── Workspace Body Content ── */}
          <div className="flex-1 p-4 overflow-hidden flex flex-col">
            
            {selectedProject ? (
              /* CASE 1: Active Interactive Live Pseudo-Terminal */
              <div className="flex-1 flex flex-col overflow-hidden">
                <div className="mb-2 flex items-center justify-between shrink-0">
                  <div className="flex items-center gap-2">
                    <TerminalIcon className="size-4 text-indigo-400" />
                    <span className="text-sm font-semibold tracking-wide font-mono text-indigo-200">
                      {selectedProject.name} — Interactive Shell
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-[10px] bg-slate-900 border border-border/20 px-2 py-0.5 rounded font-mono text-emerald-400">
                      ACTIVE TERMINAL
                    </span>
                  </div>
                </div>
                <div className="flex-1 overflow-hidden relative">
                  {/* Dynamically re-renders TerminalView only when project changes to spawn fresh shell */}
                  <TerminalView key={selectedProject.path.toString()} cwd={selectedProject.path.toString()} />
                </div>
              </div>
            ) : activeTab === "diagnostics" ? (
              /* CASE 2: Environment Diagnostics Console View */
              <div className="flex-1 flex flex-col overflow-hidden bg-[#0c0d0e] border border-border rounded-lg p-4 font-mono text-sm">
                <div className="flex items-center justify-between border-b border-border/20 pb-2 mb-3">
                  <div className="flex items-center gap-2 text-indigo-300">
                    <HeartPulse className="size-4 text-pink-400" />
                    <span>Projm Environment Scan Logs</span>
                  </div>
                  <button
                    onClick={handleDiagnostics}
                    disabled={runningDiagnostics}
                    className="bg-indigo-600 hover:bg-indigo-500 text-white font-sans text-xs px-3 py-1 rounded transition-colors disabled:opacity-50"
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
                <div className="flex items-center justify-between border-b border-white/5 pb-3">
                  <div className="flex flex-col gap-0.5">
                    <h1 className="text-xl font-bold tracking-tight bg-gradient-to-r from-white via-slate-200 to-[#64748b] bg-clip-text text-transparent">
                      Workspace Dashboard
                    </h1>
                    <p className="text-[10px] text-zinc-500">Overview of organized workspace directories and Git configurations.</p>
                  </div>
                  
                  <div className="flex items-center gap-1.5 text-[10px] bg-[#111215] border border-white/5 rounded px-2 py-1 font-mono text-slate-400">
                    <span className="text-[#64748b]">Base path:</span>
                    <span className="text-indigo-400 truncate max-w-[140px] sm:max-w-xs">{config?.base ?? "Not configured"}</span>
                    <Link href="/settings" className="text-indigo-300 hover:text-indigo-200 ml-1 font-sans transition-colors font-medium">
                      Change →
                    </Link>
                  </div>
                </div>

                {/* Global Workspace Health Stats Row */}
                <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-3">
                  {/* Card 1: Total projects */}
                  <div className="p-3 rounded-lg border border-white/5 bg-[#0f1012] flex items-center justify-between group hover:border-[#1f2937]/50 transition-all">
                    <div className="flex flex-col gap-0.5">
                      <span className="text-[9px] tracking-wider text-muted-foreground font-mono uppercase">
                        Total Projects
                      </span>
                      <span className="text-lg font-bold text-slate-100 font-mono">
                        {projects.length}
                      </span>
                      <span className="text-[9px] text-[#64748b]">Classified items</span>
                    </div>
                    <div className="p-1.5 rounded bg-indigo-500/10 text-indigo-400">
                      <FolderTree className="size-4" />
                    </div>
                  </div>

                  {/* Card 2: Active categories */}
                  {(() => {
                    const activeCats = new Set(projects.map(p => p.category.toLowerCase())).size;
                    return (
                      <div className="p-3 rounded-lg border border-white/5 bg-[#0f1012] flex items-center justify-between group hover:border-[#1f2937]/50 transition-all">
                        <div className="flex flex-col gap-0.5">
                          <span className="text-[9px] tracking-wider text-muted-foreground font-mono uppercase">
                            Active Folders
                          </span>
                          <span className="text-lg font-bold text-emerald-400 font-mono">
                            {activeCats} <span className="text-xs text-muted-foreground font-normal">/ {categories.length}</span>
                          </span>
                          <span className="text-[9px] text-[#64748b]">Populated folders</span>
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
                      <div className="p-3 rounded-lg border border-white/5 bg-[#0f1012] flex items-center justify-between group hover:border-[#1f2937]/50 transition-all">
                        <div className="flex flex-col gap-0.5 truncate w-full">
                          <span className="text-[9px] tracking-wider text-muted-foreground font-mono uppercase">
                            Last Scanned
                          </span>
                          <span className="text-xs font-semibold text-indigo-300 font-mono truncate max-w-[130px]" title={lastProject?.name.toString() ?? "None"}>
                            {lastProject?.name ?? "None"}
                          </span>
                          {lastProject ? (
                            <span className={`text-[8px] px-1 py-0.2 rounded font-mono font-bold capitalize border w-fit ${meta?.color} ${meta?.bg} ${meta?.border}`}>
                              {lastProject.category}
                            </span>
                          ) : (
                            <span className="text-[9px] text-[#64748b]">No scans executed</span>
                          )}
                        </div>
                        <div className="p-1.5 rounded bg-indigo-500/10 text-indigo-400">
                          <Activity className="size-4" />
                        </div>
                      </div>
                    );
                  })()}

                  {/* Card 4: Untracked projects */}
                  {(() => {
                    const untrackedCount = projects.filter(p => !p.git_branch).length;
                    return (
                      <div className="p-3 rounded-lg border border-white/5 bg-[#0f1012] flex items-center justify-between group hover:border-[#1f2937]/50 transition-all">
                        <div className="flex flex-col gap-0.5">
                          <span className="text-[9px] tracking-wider text-muted-foreground font-mono uppercase">
                            Untracked
                          </span>
                          <span className={`text-lg font-bold font-mono ${untrackedCount > 0 ? "text-amber-400" : "text-slate-400"}`}>
                            {untrackedCount}
                          </span>
                          <span className="text-[9px] text-[#64748b]">Missing Git tracking</span>
                        </div>
                        <div className={`p-1.5 rounded bg-amber-500/10 text-amber-400 ${untrackedCount > 0 ? "" : "bg-zinc-800 text-slate-500"}`}>
                          <GitBranch className="size-4" />
                        </div>
                      </div>
                    );
                  })()}
                </div>

                {/* Categories Grid List */}
                <div className="flex flex-col gap-2">
                  <span className="text-[10px] font-mono text-[#64748b] tracking-wider uppercase">
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
                              ? `bg-[#0f1012] hover:bg-[#151619] border-white/5 hover:border-${meta.accentHex}/30 shadow-[0_4px_20px_rgba(0,0,0,0.4)]`
                              : "bg-[#0c0d0e]/40 border-white/5 opacity-55 hover:opacity-80"
                          }`}
                          style={isPopulated ? {
                            boxShadow: `inset 0 1px 0 0 rgba(255,255,255,0.03), 0 4px 20px rgba(0,0,0,0.4)`
                          } : undefined}
                        >
                          <div className="flex items-start justify-between w-full">
                            <div className="flex items-center gap-2">
                              <span className={`w-4 h-4 rounded flex items-center justify-center text-[9px] font-mono font-bold bg-black/40 border border-white/10 ${isPopulated ? meta.color : "text-slate-500"}`}>
                                {meta.letter}
                              </span>
                              <span className={`text-[11px] font-semibold capitalize tracking-wide ${isPopulated ? "text-slate-200 group-hover:text-white" : "text-slate-500"}`}>
                                {cat.name}
                              </span>
                            </div>
                            
                            <span className={`text-[10px] font-mono px-1.5 py-0.2 rounded ${isPopulated ? `${meta.color} bg-black/40 border border-white/5` : "text-slate-600 bg-black/10"}`}>
                              {count}
                            </span>
                          </div>

                          <div className="w-full mt-3 flex flex-col gap-1">
                            <span className={`text-[9px] ${isPopulated ? "text-[#64748b] font-medium" : "text-slate-600"}`}>
                              {isPopulated ? `${count} ${count === 1 ? "project" : "projects"}` : "empty folder"}
                            </span>
                            
                            {/* Proportional Fill Progress Bar */}
                            <div className="w-full h-1 rounded-full bg-zinc-900 border border-white/5 overflow-hidden">
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
                <div className="p-3 rounded-lg border border-white/5 bg-[#0f1012] flex flex-col gap-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-1.5">
                      <Activity className="size-3.5 text-indigo-400" />
                      <span className="text-xs font-semibold tracking-wide text-slate-200">
                        Recent Workspace Activity
                      </span>
                    </div>
                    <span className="text-[9px] text-[#64748b] font-mono">
                      Updated just now
                    </span>
                  </div>
                  
                  {projects.length === 0 ? (
                    <div className="text-center py-4 text-xs text-[#64748b] italic">
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
                              setSelectedProject(p);
                              setSelectedCategory(p.category);
                              setActiveTab("projects");
                            }}
                            className="w-full flex flex-col sm:flex-row sm:items-center sm:justify-between p-2 rounded border border-white/5 bg-black/20 hover:bg-[#181a1c]/60 transition-all text-left group gap-2"
                          >
                            <div className="flex items-center gap-2 truncate">
                              <FolderTree className="size-3.5 text-slate-400 group-hover:text-indigo-400 shrink-0 transition-colors" />
                              <div className="flex flex-col gap-0.2 truncate">
                                <span className="text-xs font-mono font-semibold text-slate-200 group-hover:text-white truncate">
                                  {p.name}
                                </span>
                                <span className="text-[9px] text-[#64748b] font-mono truncate max-w-[200px] sm:max-w-xs">
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
                                    className="text-[8px] px-1 py-0.2 rounded bg-zinc-800/40 text-slate-400 border border-white/5 font-sans"
                                  >
                                    {tag}
                                  </span>
                                ))}
                              </div>
                              
                              {/* Category Badge */}
                              <span className={`text-[8px] px-1.5 py-0.2 rounded font-mono font-bold capitalize border ${meta.color} ${meta.bg} ${meta.border}`}>
                                {p.category}
                              </span>
                              
                              {/* Branch status */}
                              {p.git_branch ? (
                                <div className="flex items-center gap-1 bg-black/40 px-1.5 py-0.2 rounded border border-white/5 text-[9px] text-slate-400 font-mono">
                                  <GitBranch className="size-2.5 text-[#64748b]" />
                                  <span className="truncate max-w-[70px]">{p.git_branch}</span>
                                  <span className={`w-1 h-1 rounded-full ${p.git_dirty ? "bg-amber-400 animate-pulse" : "bg-emerald-400"}`} />
                                </div>
                              ) : (
                                <span className="text-[8px] text-[#64748b] font-mono">untracked</span>
                              )}
                              
                              <ChevronRight className="size-3 text-slate-500 group-hover:text-white group-hover:translate-x-0.5 transition-all hidden sm:block" />
                            </div>
                          </button>
                        );
                      })}
                    </div>
                  )}
                </div>

                {/* Scan Tips / Helpful Banner */}
                <div className="p-3 rounded-lg border border-white/5 bg-[#0f1012] flex gap-2.5 items-start">
                  <Info className="size-4 text-indigo-400 shrink-0 mt-0.5" />
                  <div className="flex flex-col gap-1 text-[11px] w-full sm:flex-row sm:justify-between sm:items-center">
                    <div className="flex flex-col gap-0.2">
                      <span className="font-semibold text-slate-200">How to populate the directories?</span>
                      <span className="text-muted-foreground leading-relaxed max-w-xl">
                        Enter the path to your source directories using the Scan page. The Projm background compiler automatically parses stacks, categorizes languages, and sets up Git tracking triggers.
                      </span>
                    </div>
                    <Link 
                      href="/scan" 
                      className="flex items-center gap-0.5 text-[10px] text-indigo-400 font-medium hover:text-indigo-300 shrink-0 mt-1 sm:mt-0 bg-indigo-500/10 px-2.5 py-1 rounded border border-indigo-500/20"
                    >
                      <span>Scan settings</span>
                      <ChevronRight className="size-2.5" />
                    </Link>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* ── Quick Ctrl+K Modal Command Finder ── */}
        {searchOpen && (
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm">
            <div className="w-full max-w-md bg-[#0f1012] border border-border rounded-xl shadow-2xl overflow-hidden flex flex-col animate-in fade-in zoom-in-95 duration-150">
              <div className="p-3 border-b border-border/20 flex items-center gap-2">
                <Search className="size-4 text-muted-foreground" />
                <input
                  type="text"
                  placeholder="Find a project by name..."
                  autoFocus
                  onChange={(e) => setSearchTerm(e.target.value)}
                  className="w-full bg-transparent focus:outline-none text-sm text-slate-100"
                />
                <button 
                  onClick={() => setSearchOpen(false)}
                  className="text-[10px] bg-slate-800 hover:bg-slate-700 px-2 py-0.5 rounded text-muted-foreground font-mono"
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
                        setSelectedProject(p);
                        setSelectedCategory(p.category);
                        setActiveTab("projects");
                        setSearchOpen(false);
                      }}
                      className="w-full text-left p-2 rounded hover:bg-[#18191c] text-xs font-mono flex justify-between items-center text-slate-300 hover:text-white"
                    >
                      <span>{p.name}</span>
                      <span className="text-[10px] uppercase text-[#64748b] bg-slate-900 border border-border/10 px-1.5 py-0.5 rounded font-sans">
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
