"use client";

import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
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
} from "lucide-react";
import TerminalView from "@/components/ui/terminal";

interface ProjectItem {
  name: String;
  path: String;
  category: string;
  git_branch?: string;
  git_dirty?: boolean;
}

interface Config {
  base: string;
}

const CATEGORIES = [
  { id: "apps", letter: "A", name: "Apps", color: "bg-blue-500/80 border-blue-500" },
  { id: "services", letter: "S", name: "Services", color: "bg-cyan-500/80 border-cyan-500" },
  { id: "ui", letter: "U", name: "UI Components", color: "bg-purple-500/80 border-purple-500" },
  { id: "embedded", letter: "E", name: "Embedded", color: "bg-yellow-500/80 border-yellow-500" },
  { id: "ml", letter: "M", name: "Machine Learning", color: "bg-green-500/80 border-green-500" },
  { id: "tools", letter: "T", name: "CLI Tools", color: "bg-orange-500/80 border-orange-500" },
  { id: "labs", letter: "L", name: "Labs", color: "bg-red-500/80 border-red-500" },
  { id: "content", letter: "C", name: "Content Docs", color: "bg-pink-500/80 border-pink-500" },
];

export default function WorkspacePage() {
  // Navigation & Category states
  const [selectedCategory, setSelectedCategory] = useState("apps");
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

  // Load backend configurations and projects list
  async function loadData() {
    setLoading(true);
    try {
      const cfg = await invoke<Config>("cmd_get_config");
      setConfig(cfg);
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

  // Listen to keyboard shortcuts (like Ctrl+K for search)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        setSearchOpen((prev) => !prev);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

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

  // Filter projects by category and search query
  const filteredProjects = projects.filter((p) => {
    const matchCat = p.category.toLowerCase() === selectedCategory.toLowerCase();
    const matchSearch = p.name.toLowerCase().includes(searchTerm.toLowerCase());
    return matchCat && matchSearch;
  });

  return (
    <div className="w-full h-screen flex bg-[#090a0b] text-[#e2e8f0] font-sans select-none">
      
      {/* ── SIDEBAR 1: Left Narrow Icon Column (48px) ── */}
      <div className="w-12 h-full flex flex-col justify-between items-center py-4 bg-[#0d0e10] border-r border-[#1f2937]/30">
        <div className="flex flex-col gap-4 items-center">
          
          {/* Top workspace logo/dashboard shortcut */}
          <button 
            onClick={() => { setSelectedProject(null); setActiveTab("projects"); }}
            className="w-8 h-8 rounded-lg flex items-center justify-center bg-[#18191b] border border-border/20 text-indigo-400 hover:text-indigo-300 hover:bg-[#202124] transition-all"
            title="Dashboard Overview"
          >
            <Sparkles className="size-4" />
          </button>
          
          <div className="w-6 h-px bg-border/20 my-1" />

          {/* Category Avatars */}
          {CATEGORIES.map((cat) => {
            const isSelected = selectedCategory === cat.id && activeTab === "projects";
            return (
              <button
                key={cat.id}
                onClick={() => {
                  setSelectedCategory(cat.id);
                  setActiveTab("projects");
                }}
                className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold transition-all relative border ${
                  isSelected
                    ? `${cat.color} text-white shadow-md scale-105`
                    : "bg-[#181a1c] border-transparent text-[#94a3b8] hover:text-white hover:bg-[#242629]"
                }`}
                title={cat.name}
              >
                {cat.letter}
                {/* Active indicator dot */}
                {isSelected && (
                  <span className="absolute -left-1 top-2.5 w-1 h-2 rounded-r-md bg-white" />
                )}
              </button>
            );
          })}
        </div>

        {/* Bottom Gear & Help Icons */}
        <div className="flex flex-col gap-3 items-center">
          <button 
            onClick={loadData}
            className="text-muted-foreground hover:text-white transition-colors" 
            title="Reload Workspace"
          >
            <RefreshCw className="size-4 animate-hover" />
          </button>
          <button className="text-muted-foreground hover:text-white transition-colors" title="Settings">
            <Settings className="size-4" />
          </button>
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

        {/* Project Folder List */}
        <div className="flex-1 overflow-y-auto px-2 py-3 flex flex-col gap-0.5">
          {loading ? (
            <div className="flex items-center justify-center h-20 text-xs text-muted-foreground">
              Loading projects...
            </div>
          ) : filteredProjects.length === 0 ? (
            <div className="text-center py-6 text-xs text-muted-foreground flex flex-col items-center gap-2">
              <FolderOpen className="size-6 text-[#475569] stroke-1" />
              <span>No projects in this category</span>
            </div>
          ) : (
            filteredProjects.map((p) => {
              const isSelected = selectedProject?.path === p.path;
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

          {/* Centered Search/Command Bar Prompt */}
          <div 
            onClick={() => setSearchOpen(true)}
            className="hidden sm:flex items-center justify-between px-3 py-1 bg-[#17181c] border border-border/10 rounded-md w-64 text-[#64748b] text-[11px] cursor-pointer hover:border-border/30 hover:text-slate-400 transition-all"
          >
            <div className="flex items-center gap-1.5">
              <Search className="size-3 text-muted-foreground" />
              <span>Search workspace</span>
            </div>
            <span className="font-mono text-[9px] bg-[#24252a] px-1 py-0.5 rounded border border-border/5">Ctrl+K</span>
          </div>

          {/* Right Header Operations */}
          <div className="flex items-center gap-2">
            <button className="p-1.5 rounded hover:bg-[#18191b] text-muted-foreground hover:text-white" title="Quick Actions">
              <Play className="size-3.5 text-emerald-400 fill-emerald-400/20" />
            </button>
            <button className="p-1.5 rounded hover:bg-[#18191b] text-[#64748b] hover:text-white">
              <Settings2 className="size-3.5" />
            </button>
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
            <div className="flex-1 overflow-y-auto flex flex-col gap-6 max-w-4xl mx-auto w-full py-4 scrollbar-thin">
              <div>
                <h1 className="text-2xl font-bold tracking-tight bg-gradient-to-r from-white via-slate-200 to-[#64748b] bg-clip-text text-transparent">
                  Workspace Dashboard
                </h1>
                <p className="text-muted-foreground text-xs mt-1">
                  System overview of your automatically classified projects
                </p>
              </div>

              {/* Grid cards */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                
                {/* Config base info */}
                <div className="p-4 rounded-xl border border-border/10 bg-[#0f1012] flex flex-col gap-1">
                  <span className="text-[10px] tracking-wider text-muted-foreground font-mono uppercase">
                    Configured Base Location
                  </span>
                  <span className="text-sm font-semibold font-mono text-indigo-300 truncate">
                    {config?.base ?? "Not configured"}
                  </span>
                </div>

                {/* Total scanned projects count */}
                <div className="p-4 rounded-xl border border-border/10 bg-[#0f1012] flex flex-col gap-1">
                  <span className="text-[10px] tracking-wider text-muted-foreground font-mono uppercase">
                    Scanned & Grouped Projects
                  </span>
                  <span className="text-sm font-semibold text-emerald-400 font-mono">
                    {projects.length} Total Projects
                  </span>
                </div>
              </div>

              {/* Categories grid list */}
              <div className="flex flex-col gap-3">
                <span className="text-[11px] font-mono text-muted-foreground tracking-wider uppercase">
                  Classified Categories Status
                </span>
                <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                  {CATEGORIES.map((cat) => {
                    const count = projects.filter((p) => p.category.toLowerCase() === cat.id.toLowerCase()).length;
                    return (
                      <button
                        key={cat.id}
                        onClick={() => {
                          setSelectedCategory(cat.id);
                          setActiveTab("projects");
                        }}
                        className="p-3 rounded-lg border border-border/15 bg-[#0f1012] hover:bg-[#151619] transition-all text-left flex items-center justify-between group"
                      >
                        <div className="flex items-center gap-2.5">
                          <span className="w-5 h-5 rounded-full flex items-center justify-center text-[10px] bg-slate-900 border border-border/10 font-bold group-hover:text-white">
                            {cat.letter}
                          </span>
                          <span className="text-xs font-semibold capitalize tracking-wide text-slate-300 group-hover:text-white">
                            {cat.id}
                          </span>
                        </div>
                        <span className="text-xs font-mono bg-slate-900/60 border border-border/10 px-2 py-0.5 rounded text-indigo-300">
                          {count}
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* Scan tips / instructions */}
              <div className="p-4 rounded-xl border border-border/10 bg-[#0f1012] flex gap-3.5">
                <Info className="size-5 text-indigo-400 shrink-0" />
                <div className="flex flex-col gap-1 text-xs">
                  <span className="font-semibold text-slate-200">How to populate the directories?</span>
                  <span className="text-muted-foreground leading-relaxed">
                    Enter the path to your source directories using the <a href="/scan" className="text-indigo-400 underline hover:text-indigo-300">Scan page</a>. The Projm background compiler automatically parses stacks, categorizes languages, and sets up Git tracking triggers.
                  </span>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* ── Quick Ctrl+K Modal Command Finder ── */}
      {searchOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm">
          <div className="w-full max-w-md bg-[#0f1012] border border-border rounded-xl shadow-2xl overflow-hidden flex flex-col">
            <div className="p-3 border-b border-border/20 flex items-center gap-2">
              <Search className="size-4 text-muted-foreground" />
              <input
                type="text"
                placeholder="Find a project by name..."
                autoFocus
                onChange={(e) => setSearchTerm(e.target.value)}
                className="w-full bg-transparent focus:outline-none text-sm"
              />
              <button 
                onClick={() => setSearchOpen(false)}
                className="text-[10px] bg-slate-800 hover:bg-slate-700 px-2 py-0.5 rounded text-muted-foreground"
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
                    className="w-full text-left p-2 rounded hover:bg-[#18191c] text-xs font-mono flex justify-between items-center"
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
  );
}
