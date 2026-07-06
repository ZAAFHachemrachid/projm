"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useHotkey } from "@tanstack/react-hotkeys";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  FolderOpen,
  Code,
  Info,
  Save,
  Check,
  AlertCircle,
  Plus,
  Trash2,
  Folder,
  ShieldAlert,
  X,
  RefreshCw,
  Eye,
  Sliders,
  ChevronUp,
  ChevronDown,
  Layers,
  Sparkles,
  BookOpen,
  ArrowLeft,
  Bot,
  Terminal,
} from "lucide-react";

interface Config {
  base: string;
  categories: string[];
}

interface Editor {
  binary: string;
  name: string;
  path: string;
}

interface AgentInfo {
  name: string;
  command: string;
  binary: string;
  installed: boolean;
  path: string | null;
}

interface CustomRule {
  name?: string;
  name_contains?: string;
  marker?: string;
  suffix?: string;
  has_dep?: string;
  category: string;
}

// Simple robust client-side TOML parser for [[rule]] blocks
function parseRulesToml(toml: string): CustomRule[] {
  const rules: CustomRule[] = [];
  const blocks = toml.split(/\[\[rule\]\]/i);
  for (let i = 1; i < blocks.length; i++) {
    const block = blocks[i];
    const rule: Partial<CustomRule> = {};
    const lines = block.split("\n");
    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith("#")) continue;
      // Capture key and quoted value, ignoring trailing spaces or comments
      const match = trimmed.match(/^([a-zA-Z0-9-_]+)\s*=\s*["']([^"']+)["']/);
      if (match) {
        const key = match[1];
        const value = match[2];
        if (["name", "name_contains", "marker", "suffix", "has_dep", "category"].includes(key)) {
          rule[key as keyof CustomRule] = value;
        }
      }
    }
    if (rule.category) {
      rules.push(rule as CustomRule);
    }
  }
  return rules;
}

// Stringifies CustomRule array back into rules.toml syntax
function stringifyRulesToml(rules: CustomRule[]): string {
  let toml = `# ==============================================================================\n`;
  toml += `# Projm Custom Classification Rules Configuration (rules.toml)\n`;
  toml += `# ==============================================================================\n`;
  toml += `#\n`;
  toml += `# Rules are evaluated from top to bottom. The first matching rule wins.\n`;
  toml += `# Within a single [[rule]], all specified criteria must match (AND logic).\n`;
  toml += `#\n`;
  toml += `# Supported fields:\n`;
  toml += `# - name          : Exact name match of the project directory (e.g. "pioneers-website")\n`;
  toml += `# - name_contains : Substring match of the project directory name (e.g. "adrar")\n`;
  toml += `# - marker        : File/directory presence marker in the project root (e.g. "rocket.toml")\n`;
  toml += `# - suffix        : Override built-in suffix behaviour (e.g. "fw")\n`;
  toml += `# - has_dep       : Check dependencies in Cargo.toml, package.json, or requirements.txt (e.g. "burn")\n`;
  toml += `#\n\n`;

  for (const rule of rules) {
    toml += `[[rule]]\n`;
    if (rule.name) toml += `name = "${rule.name}"\n`;
    if (rule.name_contains) toml += `name_contains = "${rule.name_contains}"\n`;
    if (rule.marker) toml += `marker = "${rule.marker}"\n`;
    if (rule.suffix) toml += `suffix = "${rule.suffix}"\n`;
    if (rule.has_dep) toml += `has_dep = "${rule.has_dep}"\n`;
    toml += `category = "${rule.category}"\n\n`;
  }
  return toml;
}

// Semantic category colors and visual styling mapping
const semanticCategoryStyles: Record<
  string,
  { dot: string; text: string; bg: string; border: string; desc: string }
> = {
  apps: {
    dot: "bg-indigo-500 shadow-[0_0_8px_rgba(99,102,241,0.6)]",
    text: "text-indigo-400",
    border: "hover:border-indigo-500/30 border-white/5",
    bg: "bg-indigo-500/5",
    desc: "Application Workspace",
  },
  services: {
    dot: "bg-cyan-500 shadow-[0_0_8px_rgba(6,182,212,0.6)]",
    text: "text-cyan-400",
    border: "hover:border-cyan-500/30 border-white/5",
    bg: "bg-cyan-500/5",
    desc: "Backend Services",
  },
  ui: {
    dot: "bg-fuchsia-500 shadow-[0_0_8px_rgba(217,70,239,0.6)]",
    text: "text-fuchsia-400",
    border: "hover:border-fuchsia-500/30 border-white/5",
    bg: "bg-fuchsia-500/5",
    desc: "Design & Frontend",
  },
  embedded: {
    dot: "bg-amber-500 shadow-[0_0_8px_rgba(245,158,11,0.6)]",
    text: "text-amber-400",
    border: "hover:border-amber-500/30 border-white/5",
    bg: "bg-amber-500/5",
    desc: "Firmware & IoT",
  },
  ml: {
    dot: "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.6)]",
    text: "text-emerald-400",
    border: "hover:border-emerald-500/30 border-white/5",
    bg: "bg-emerald-500/5",
    desc: "Machine Learning / AI",
  },
  tools: {
    dot: "bg-zinc-400 shadow-[0_0_8px_rgba(161,161,170,0.6)]",
    text: "text-zinc-300",
    border: "hover:border-zinc-500/30 border-white/5",
    bg: "bg-zinc-500/5",
    desc: "CLI & System Tools",
  },
  labs: {
    dot: "bg-orange-500 shadow-[0_0_8px_rgba(249,115,22,0.6)]",
    text: "text-orange-400",
    border: "hover:border-orange-500/30 border-white/5",
    bg: "bg-orange-500/5",
    desc: "Experimental Sandbox",
  },
  content: {
    dot: "bg-pink-500 shadow-[0_0_8px_rgba(236,72,153,0.6)]",
    text: "text-pink-400",
    border: "hover:border-pink-500/30 border-white/5",
    bg: "bg-pink-500/5",
    desc: "Media, Content & Docs",
  },
};

const customCategoryStyles = [
  {
    dot: "bg-violet-500 shadow-[0_0_8px_rgba(139,92,246,0.6)]",
    text: "text-violet-400",
    border: "hover:border-violet-500/30 border-white/5",
    bg: "bg-violet-500/5",
    desc: "Custom Folder",
  },
  {
    dot: "bg-teal-500 shadow-[0_0_8px_rgba(20,184,166,0.6)]",
    text: "text-teal-400",
    border: "hover:border-teal-500/30 border-white/5",
    bg: "bg-teal-500/5",
    desc: "Custom Folder",
  },
];

function getCategoryStyle(catName: string, index: number) {
  const norm = catName.toLowerCase().trim();
  if (semanticCategoryStyles[norm]) {
    return semanticCategoryStyles[norm];
  }
  return customCategoryStyles[index % customCategoryStyles.length];
}

// Right Panel Directory Tree Simulator Component
function DirectoryTree({ baseDir, categories }: { baseDir: string; categories: string[] }) {
  return (
    <Card className="border border-white/5 bg-zinc-950/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden h-full">
      <CardHeader className="pb-4 border-b border-white/5 bg-zinc-950/20">
        <CardTitle className="text-xs font-bold tracking-widest uppercase text-indigo-400 flex items-center gap-2.5">
          <Eye className="size-4 text-indigo-400" />
          Workspace Tree Preview
        </CardTitle>
      </CardHeader>
      <CardContent className="pt-6 space-y-4 font-mono text-xs">
        <p className="text-xs font-sans text-zinc-400 leading-relaxed mb-2">
          Simulator displaying how categorized folders are structured on disk in real time:
        </p>

        <div className="bg-black/40 border border-white/5 p-5 rounded-lg space-y-2 select-all max-h-[380px] overflow-y-auto pr-2 scrollbar-thin scrollbar-thumb-zinc-800 scrollbar-track-transparent">
          <div className="flex items-center gap-2 text-indigo-400 font-bold">
            <span>📁</span>
            <span>{baseDir || "~/projects"}</span>
          </div>

          {categories.length === 0 ? (
            <div className="text-zinc-600 pl-4 py-2 italic">
              No custom category folders.
            </div>
          ) : (
            categories.map((cat, idx) => {
              const style = getCategoryStyle(cat, idx);
              return (
                <div key={cat} className="flex items-center gap-2 pl-4 py-0.5 group">
                  <span className="text-zinc-700/80">├──</span>
                  <span>📁</span>
                  <span className={`capitalize font-semibold ${style.text}`}>{cat}/</span>
                </div>
              );
            })
          )}
          {/* Always display warning fallback folder */}
          <div className="flex items-center gap-2 pl-4 py-0.5">
            <span className="text-zinc-700/80">└──</span>
            <span>📁</span>
            <span className="text-rose-400/90 font-bold">undefined/</span>
            <span className="text-[10px] text-zinc-600 font-sans italic">(Fallback sandbox)</span>
          </div>
        </div>

        <div className="p-3 bg-zinc-900/30 border border-white/5 rounded-lg text-[10px] font-sans text-zinc-500 leading-relaxed space-y-1">
          <p className="font-semibold text-zinc-400">Preview Specifications</p>
          <p>• Root directory modifies dynamically based on Workspaces setting.</p>
          <p>• Unmapped or deactivated categories fallback directly to <code className="bg-zinc-800/80 text-zinc-300 font-mono px-1 rounded border border-white/5">undefined</code> sandbox.</p>
        </div>
      </CardContent>
    </Card>
  );
}

export default function SettingsPage() {
  const router = useRouter();

  // Escape to seamlessly navigate back home
  useHotkey("Escape", () => {
    router.push("/");
  });

  const [config, setConfig] = useState<Config | null>(null);
  const [loading, setLoading] = useState(true);
  const [baseDir, setBaseDir] = useState("");
  const [categories, setCategories] = useState<string[]>([]);
  
  // Tabs active section state
  const [activeTab, setActiveTab] = useState("general");

  // Blueprints States
  const [blueprints, setBlueprints] = useState<{ name: string; command: string }[]>([]);
  const [newBlueprintName, setNewBlueprintName] = useState("");
  const [newBlueprintCommand, setNewBlueprintCommand] = useState("");
  const [savingBlueprint, setSavingBlueprint] = useState(false);
  const [blueprintError, setBlueprintError] = useState<string | null>(null);
  const [blueprintMessage, setBlueprintMessage] = useState<{ ok: boolean; text: string } | null>(null);
  const [editingBlueprint, setEditingBlueprint] = useState<string | null>(null);
  const [editBlueprintName, setEditBlueprintName] = useState("");
  const [editBlueprintCommand, setEditBlueprintCommand] = useState("");

  // AI Agents States
  const [agents, setAgents] = useState<AgentInfo[]>([]);
  const [newAgentName, setNewAgentName] = useState("");
  const [newAgentCommand, setNewAgentCommand] = useState("");
  const [savingAgent, setSavingAgent] = useState(false);
  const [agentError, setAgentError] = useState<string | null>(null);
  const [agentMessage, setAgentMessage] = useState<{ ok: boolean; text: string } | null>(null);
  const [editingAgent, setEditingAgent] = useState<string | null>(null);
  const [editAgentName, setEditAgentName] = useState("");
  const [editAgentCommand, setEditAgentCommand] = useState("");

  // Rules Editor States
  const [rulesRaw, setRulesRaw] = useState("");
  const [rulesList, setRulesList] = useState<CustomRule[]>([]);
  const [isVisualMode, setIsVisualMode] = useState(true);
  const [savingRules, setSavingRules] = useState(false);
  const [rulesMessage, setRulesMessage] = useState<{ ok: boolean; text: string } | null>(null);
  const [rulesError, setRulesError] = useState<string | null>(null);

  // Base directory save state
  const [savingBase, setSavingBase] = useState(false);
  const [baseMessage, setBaseMessage] = useState<{
    ok: boolean;
    text: string;
  } | null>(null);

  // Categories save state
  const [savingCategories, setSavingCategories] = useState(false);
  const [categoryMessage, setCategoryMessage] = useState<{
    ok: boolean;
    text: string;
  } | null>(null);

  // New category creation state
  const [newCatInput, setNewCatInput] = useState("");
  const [newCatError, setNewCatError] = useState<string | null>(null);

  // Confirmation modal states
  const [categoryToDelete, setCategoryToDelete] = useState<string | null>(null);

  const [editors, setEditors] = useState<Editor[]>([]);
  const [editorsLoading, setEditorsLoading] = useState(true);

  async function loadConfig() {
    try {
      const cfg = await invoke<Config>("cmd_get_config");
      setConfig(cfg);
      setBaseDir(cfg.base);
      setCategories(cfg.categories || []);
    } catch (err) {
      console.error("Failed to load config:", err);
    }
  }

  async function loadEditors() {
    setEditorsLoading(true);
    try {
      const list = await invoke<Editor[]>("cmd_get_editors");
      setEditors(list);
    } catch {
      // silently fail
    } finally {
      setEditorsLoading(false);
    }
  }

  async function loadRules() {
    try {
      const raw = await invoke<string>("cmd_get_rules_raw");
      setRulesRaw(raw);
      try {
        const parsed = parseRulesToml(raw);
        setRulesList(parsed);
        setRulesError(null);
      } catch (err) {
        setRulesError("Failed parsing rules.toml contents: " + err);
      }
    } catch (err) {
      console.error("Failed to load custom classification rules:", err);
    }
  }

  async function loadBlueprints() {
    try {
      const list = await invoke<{ name: string; command: string }[]>("cmd_get_blueprints");
      setBlueprints(list);
    } catch (err) {
      console.error("Failed to load blueprints:", err);
    }
  }

  async function handleAddBlueprint() {
    const name = newBlueprintName.trim();
    const command = newBlueprintCommand.trim();
    if (!name || !command) return;
    setSavingBlueprint(true);
    setBlueprintError(null);
    setBlueprintMessage(null);
    try {
      await invoke("cmd_add_blueprint", { name, command });
      await loadBlueprints();
      setNewBlueprintName("");
      setNewBlueprintCommand("");
      setBlueprintMessage({ ok: true, text: `Blueprint '${name}' added successfully!` });
    } catch (err) {
      setBlueprintError(String(err));
      setBlueprintMessage({ ok: false, text: "Failed to add blueprint." });
    } finally {
      setSavingBlueprint(false);
      setTimeout(() => setBlueprintMessage(null), 3000);
    }
  }

  function startEditBlueprint(bp: { name: string; command: string }) {
    setEditingBlueprint(bp.name);
    setEditBlueprintName(bp.name);
    setEditBlueprintCommand(bp.command);
    setBlueprintError(null);
  }

  async function handleSaveEditBlueprint(oldName: string) {
    const name = editBlueprintName.trim();
    const command = editBlueprintCommand.trim();
    if (!name || !command) return;
    setSavingBlueprint(true);
    setBlueprintError(null);
    setBlueprintMessage(null);
    try {
      await invoke("cmd_update_blueprint", { oldName, name, command });
      await loadBlueprints();
      setEditingBlueprint(null);
      setBlueprintMessage({ ok: true, text: `Blueprint '${name}' updated!` });
    } catch (err) {
      setBlueprintError(String(err));
      setBlueprintMessage({ ok: false, text: "Failed to update blueprint." });
    } finally {
      setSavingBlueprint(false);
      setTimeout(() => setBlueprintMessage(null), 3000);
    }
  }

  async function handleDeleteBlueprint(name: string) {
    setBlueprintError(null);
    setBlueprintMessage(null);
    try {
      await invoke("cmd_delete_blueprint", { name });
      await loadBlueprints();
      setBlueprintMessage({ ok: true, text: `Blueprint '${name}' deleted.` });
    } catch (err) {
      setBlueprintError(String(err));
      setBlueprintMessage({ ok: false, text: "Failed to delete blueprint." });
    }
    setTimeout(() => setBlueprintMessage(null), 3000);
  }

  async function loadAgents() {
    try {
      const list = await invoke<AgentInfo[]>("cmd_get_agents");
      setAgents(list);
    } catch (err) {
      console.error("Failed to load AI agents:", err);
    }
  }

  async function handleAddAgent() {
    const name = newAgentName.trim();
    const command = newAgentCommand.trim();
    if (!name || !command) return;
    setSavingAgent(true);
    setAgentError(null);
    setAgentMessage(null);
    try {
      await invoke("cmd_add_agent", { name, command });
      await loadAgents();
      setNewAgentName("");
      setNewAgentCommand("");
      setAgentMessage({ ok: true, text: `Agent '${name}' added successfully!` });
    } catch (err) {
      setAgentError(String(err));
      setAgentMessage({ ok: false, text: "Failed to add agent." });
    } finally {
      setSavingAgent(false);
      setTimeout(() => setAgentMessage(null), 3000);
    }
  }

  function startEditAgent(agent: AgentInfo) {
    setEditingAgent(agent.name);
    setEditAgentName(agent.name);
    setEditAgentCommand(agent.command);
    setAgentError(null);
  }

  async function handleSaveEditAgent(oldName: string) {
    const name = editAgentName.trim();
    const command = editAgentCommand.trim();
    if (!name || !command) return;
    setSavingAgent(true);
    setAgentError(null);
    setAgentMessage(null);
    try {
      await invoke("cmd_update_agent", { oldName, name, command });
      await loadAgents();
      setEditingAgent(null);
      setAgentMessage({ ok: true, text: `Agent '${name}' updated!` });
    } catch (err) {
      setAgentError(String(err));
      setAgentMessage({ ok: false, text: "Failed to update agent." });
    } finally {
      setSavingAgent(false);
      setTimeout(() => setAgentMessage(null), 3000);
    }
  }

  async function handleDeleteAgent(name: string) {
    setAgentError(null);
    setAgentMessage(null);
    try {
      await invoke("cmd_delete_agent", { name });
      await loadAgents();
      setAgentMessage({ ok: true, text: `Agent '${name}' deleted.` });
    } catch (err) {
      setAgentError(String(err));
      setAgentMessage({ ok: false, text: "Failed to delete agent." });
    }
    setTimeout(() => setAgentMessage(null), 3000);
  }

  useEffect(() => {
    async function init() {
      setLoading(true);
      await Promise.all([loadConfig(), loadEditors(), loadRules(), loadBlueprints(), loadAgents()]);
      setLoading(false);
    }
    init();
    // Deep link: /settings?tab=agents opens directly on the requested tab.
    const tab = new URLSearchParams(window.location.search).get("tab");
    if (tab) setActiveTab(tab);
  }, []);

  async function handleSaveBase() {
    if (!baseDir.trim()) return;
    setSavingBase(true);
    setBaseMessage(null);
    try {
      await invoke("cmd_set_base", { path: baseDir.trim() });
      setConfig((prev) => prev ? { ...prev, base: baseDir.trim() } : null);
      setBaseMessage({ ok: true, text: "Base directory saved successfully" });
    } catch (err) {
      setBaseMessage({ ok: false, text: `Failed to save: ${err}` });
    } finally {
      setSavingBase(false);
      setTimeout(() => setBaseMessage(null), 3000);
    }
  }

  async function handleBrowseFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: baseDir || undefined,
        title: "Select Workspaces Base Directory",
      });
      if (selected && typeof selected === "string") {
        setBaseDir(selected);
        setBaseMessage(null);
      }
    } catch (err) {
      console.error("Folder picker error:", err);
    }
  }

  async function handleSaveCategories(updatedCategories: string[]) {
    setSavingCategories(true);
    setCategoryMessage(null);
    try {
      await invoke("cmd_set_categories", { categories: updatedCategories });
      setConfig((prev) => prev ? { ...prev, categories: updatedCategories } : null);
      setCategories(updatedCategories);
      setCategoryMessage({ ok: true, text: "Folder categories saved successfully" });
    } catch (err) {
      setCategoryMessage({ ok: false, text: `Failed to save folders: ${err}` });
    } finally {
      setSavingCategories(false);
      setTimeout(() => setCategoryMessage(null), 3000);
    }
  }

  function handleAddCategory() {
    const formatted = newCatInput.trim().toLowerCase();
    if (!formatted) return;

    // Validation checks
    if (formatted === "undefined") {
      setNewCatError("Folder name 'undefined' is reserved for unrecognized mappings");
      return;
    }
    if (categories.includes(formatted)) {
      setNewCatError("This folder category already exists");
      return;
    }
    if (!/^[a-z0-9-_]+$/.test(formatted)) {
      setNewCatError("Only alphanumeric characters, dashes, and underscores are allowed");
      return;
    }

    const updated = [...categories, formatted];
    setCategories(updated);
    setNewCatInput("");
    setNewCatError(null);
    handleSaveCategories(updated);
  }

  function handleRemoveCategory(cat: string) {
    // Show warning prompt before deletion
    setCategoryToDelete(cat);
  }

  function confirmDeleteCategory() {
    if (!categoryToDelete) return;
    const updated = categories.filter((c) => c !== categoryToDelete);
    setCategories(updated);
    setCategoryToDelete(null);
    handleSaveCategories(updated);
  }

  // Dual-mode transition helper for Rules configuration
  const handleToggleRulesMode = (visual: boolean) => {
    if (visual) {
      try {
        const parsed = parseRulesToml(rulesRaw);
        setRulesList(parsed);
        setRulesError(null);
        setIsVisualMode(true);
      } catch (err) {
        setRulesError("Cannot switch to Visual Designer: Invalid raw TOML syntax. Please correct code errors first.");
      }
    } else {
      const stringified = stringifyRulesToml(rulesList);
      setRulesRaw(stringified);
      setRulesError(null);
      setIsVisualMode(false);
    }
  };

  // Rule operations: Edit, Delete, Move (priority adjustments)
  const handleUpdateRuleField = (index: number, field: keyof CustomRule, value: string) => {
    const updated = [...rulesList];
    if (value === "") {
      delete updated[index][field];
    } else {
      updated[index] = { ...updated[index], [field]: value };
    }
    setRulesList(updated);
  };

  const handleAddBlankRule = () => {
    const newRule: CustomRule = { category: categories[0] || "apps" };
    setRulesList([...rulesList, newRule]);
  };

  const handleRemoveRule = (index: number) => {
    const updated = rulesList.filter((_, idx) => idx !== index);
    setRulesList(updated);
  };

  const handleMoveRule = (index: number, direction: "up" | "down") => {
    if (direction === "up" && index === 0) return;
    if (direction === "down" && index === rulesList.length - 1) return;

    const targetIdx = direction === "up" ? index - 1 : index + 1;
    const updated = [...rulesList];
    const temp = updated[index];
    updated[index] = updated[targetIdx];
    updated[targetIdx] = temp;
    setRulesList(updated);
  };

  // Save rules handler (supports both visual and raw modes)
  const handleSaveBlueprintRules = async () => {
    setSavingRules(true);
    setRulesMessage(null);
    setRulesError(null);

    let contentToSave = rulesRaw;
    if (isVisualMode) {
      contentToSave = stringifyRulesToml(rulesList);
      setRulesRaw(contentToSave);
    }

    try {
      // Validate that it parses as correct TOML in backend
      await invoke("cmd_save_rules_raw", { content: contentToSave });
      
      // Parse again locally just to keep list perfectly in sync
      const parsed = parseRulesToml(contentToSave);
      setRulesList(parsed);

      setRulesMessage({ ok: true, text: "Blueprint rules saved successfully!" });
    } catch (err) {
      setRulesError(`Failed saving configuration: ${err}`);
      setRulesMessage({ ok: false, text: "Failed to persist changes." });
    } finally {
      setSavingRules(false);
      setTimeout(() => setRulesMessage(null), 4000);
    }
  };

  const editorIcons: Record<string, string> = {
    nvim: "",
    code: "⬡",
    cursor: "▽",
    zed: "✦",
    hx: "◆",
    idea: "◈",
    emacs: "◇",
    vim: "▴",
    kiro: "◉",
  };

  const tabs = [
    { id: "general", label: "General", icon: Sliders },
    { id: "categories", label: "Categories", icon: Layers },
    { id: "blueprints", label: "Blueprints", icon: Sparkles },
    { id: "agents", label: "AI Agents", icon: Bot },
    { id: "rules", label: "Rules Config", icon: BookOpen },
    { id: "engine", label: "Engine Specs", icon: Info },
  ];

  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center min-h-[400px] space-y-4">
        <RefreshCw className="size-8 text-primary animate-spin" />
        <p className="text-sm text-muted-foreground animate-pulse">Loading settings panel...</p>
      </div>
    );
  }

  return (
    <div className="h-full overflow-hidden flex flex-col w-full max-w-7xl mx-auto px-6 lg:px-8 pt-8 lg:pt-12">
      {/* Title Header with centered spacing and nice boundaries */}
      <div className="border-b border-white/5 pb-4 shrink-0 flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-extrabold tracking-tight bg-gradient-to-r from-white via-zinc-200 to-zinc-400 bg-clip-text text-transparent flex items-center gap-3">
            <Link href="/" className="group inline-flex items-center justify-center p-1.5 rounded-lg border border-white/5 bg-zinc-900/60 text-zinc-400 hover:text-white hover:border-white/10 hover:bg-white/5 transition-all duration-200 mr-1" title="Back to home">
              <ArrowLeft className="size-5 transition-transform group-hover:-translate-x-0.5" />
            </Link>
            Settings
          </h1>
          <p className="text-xs text-muted-foreground mt-1 leading-relaxed">
            Configure workspaces, folder categories, classification rules, and project blueprints.
          </p>
        </div>
      </div>

      {/* Tabs Pill Navigation Shell */}
      <div className="flex items-center gap-2 mt-5 mb-6 border-b border-white/5 pb-3 shrink-0">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          const isActive = activeTab === tab.id;
          return (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center gap-2 px-4 py-2 text-xs font-semibold rounded-lg transition-all duration-200 ${
                isActive
                  ? "bg-indigo-600/15 text-indigo-400 border border-indigo-500/20 shadow-[0_0_10px_rgba(99,102,241,0.08)]"
                  : "text-zinc-400 hover:text-white border border-transparent hover:bg-white/5"
              }`}
            >
              <Icon className="size-3.5" />
              {tab.label}
            </button>
          );
        })}
      </div>

      {/* Dual Column Layout (Left: Settings, Right: Tree Preview) */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 items-start flex-1 overflow-hidden pb-8">
        
        {/* Left Column - Scrollable Tab Content Container */}
        <div className="lg:col-span-7 h-full overflow-y-auto pr-1 scrollbar-thin scrollbar-thumb-zinc-800 scrollbar-track-transparent space-y-6">

          {/* TAB 1: GENERAL SETTINGS */}
          {activeTab === "general" && (
            <div className="space-y-6 animate-in fade-in duration-200">
              {/* Base Directory Configuration Card */}
              <Card className="border border-white/5 bg-zinc-950/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden transition-all duration-300 hover:border-white/10">
                <CardHeader className="p-6 pb-4">
                  <CardTitle className="text-xs font-bold tracking-widest uppercase text-indigo-400 flex items-center gap-2.5">
                    <FolderOpen className="size-4.5 text-indigo-400" />
                    Base Workspaces Directory
                  </CardTitle>
                </CardHeader>
                <CardContent className="p-6 pt-0 space-y-5">
                  <p className="text-sm text-zinc-400 leading-relaxed">
                    All organized developer projects are structured under this root path. Changing this will update the destination for future sorting operations.
                  </p>
                  <div className="flex gap-3">
                    <div className="relative flex-1">
                      <Input
                        value={baseDir}
                        onChange={(e) => {
                          setBaseDir(e.target.value);
                          setBaseMessage(null);
                        }}
                        className="font-mono text-sm border-white/5 bg-black/40 h-10 pr-10 focus-visible:ring-1 focus-visible:ring-indigo-500/50"
                        placeholder="~/projects"
                      />
                      <button
                        onClick={handleBrowseFolder}
                        className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-white transition-colors"
                        title="Choose folder in file explorer"
                      >
                        <FolderOpen className="size-4" />
                      </button>
                    </div>
                    <Button
                      onClick={handleSaveBase}
                      disabled={savingBase || !baseDir.trim() || baseDir === config?.base}
                      className="bg-indigo-600 hover:bg-indigo-500 text-white font-semibold h-10 px-5 shadow-sm transition-all duration-150 active:scale-95 disabled:opacity-50 disabled:pointer-events-none"
                    >
                      {savingBase ? (
                        "Saving..."
                      ) : (
                        <>
                          <Save className="size-3.5 mr-1.5" />
                          Save Path
                        </>
                      )}
                    </Button>
                  </div>
                  <div className="flex items-center gap-2">
                    <p className="text-xs text-muted-foreground">
                      Active configuration:{" "}
                      <code className="text-xs font-mono bg-zinc-900/60 text-indigo-300 px-2 py-0.5 rounded border border-white/5">
                        {config?.base ?? "~/projects"}
                      </code>
                    </p>
                    {baseMessage && (
                      <span
                        className={`text-xs flex items-center gap-1 ${
                          baseMessage.ok ? "text-emerald-400" : "text-red-400"
                        }`}
                      >
                        {baseMessage.ok ? (
                          <Check className="size-3" />
                        ) : (
                          <AlertCircle className="size-3" />
                        )}
                        {baseMessage.text}
                      </span>
                    )}
                  </div>
                </CardContent>
              </Card>

              {/* Detected Editors Card */}
              <Card className="border border-white/5 bg-zinc-950/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden transition-all duration-300 hover:border-white/10">
                <CardHeader className="p-6 pb-4 border-b border-white/5 bg-zinc-950/20">
                  <CardTitle className="text-xs font-bold tracking-widest uppercase text-cyan-400 flex items-center gap-2.5">
                    <Code className="size-4.5 text-cyan-400" />
                    Detected System Editors
                  </CardTitle>
                </CardHeader>
                <CardContent className="p-6 space-y-4">
                  <p className="text-xs text-zinc-400 leading-relaxed">
                    These active editors are detected on your local system path. Trigger the terminal command{" "}
                    <code className="text-[10px] bg-zinc-900/60 text-cyan-300 px-1.5 py-0.5 rounded border border-white/5 font-mono">
                      projm editors
                    </code>{" "}
                    to scan and display complete bin specifications.
                  </p>

                  {editorsLoading ? (
                    <p className="text-xs text-muted-foreground animate-pulse">Detecting system installations...</p>
                  ) : editors.length === 0 ? (
                    <p className="text-xs text-amber-400 flex items-center gap-1.5">
                      <AlertCircle className="size-4.5" />
                      No supported editor installations found in $PATH.
                    </p>
                  ) : (
                    <div className="flex flex-wrap gap-2">
                      {editors.map((editor) => (
                        <Badge
                          key={editor.binary}
                          variant="secondary"
                          className="gap-2 py-1.5 px-3 border border-white/5 bg-zinc-900/50 text-[10px] font-mono text-zinc-300"
                        >
                          <span className="text-cyan-400/80 font-bold">
                            {editorIcons[editor.binary] ?? "◇"}
                          </span>
                          {editor.name}
                          {editors.length > 1 && (
                            <span className="text-[9px] text-zinc-500 font-normal">
                              ({editor.binary})
                            </span>
                          )}
                        </Badge>
                      ))}
                    </div>
                  )}

                  <Separator className="bg-white/5" />

                  <div className="text-[11px] text-zinc-500 space-y-1.5 leading-relaxed">
                    <p className="font-semibold text-zinc-300">
                      Editor Launch Mechanics
                    </p>
                    <ul className="list-disc list-inside space-y-0.5 pl-1">
                      <li>No editors detected → prompts active installation notice.</li>
                      <li>Exactly one editor found → automatically bypasses interactive prompt.</li>
                      <li>Multiple editors matched → opens dynamic picker, remembering last preference.</li>
                    </ul>
                  </div>
                </CardContent>
              </Card>
            </div>
          )}

          {/* TAB 2: CATEGORIES */}
          {activeTab === "categories" && (
            <div className="space-y-6 animate-in fade-in duration-200">
              {/* Folder Categories Manager */}
              <Card className="border border-white/5 bg-zinc-950/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden transition-all duration-300 hover:border-white/10">
                <CardHeader className="p-6 pb-4">
                  <CardTitle className="text-xs font-bold tracking-widest uppercase text-emerald-400 flex items-center gap-2.5">
                    <Folder className="size-4.5 text-emerald-400" />
                    Folder Categories Manager
                  </CardTitle>
                </CardHeader>
                <CardContent className="p-6 pt-0 space-y-6">
                  <p className="text-sm text-zinc-400 leading-relaxed">
                    Manage active folder structures inside your base directory. Projects are automatically sorted into these category folders. Removing a category redirects unrecognized items to the fallback <code className="text-xs text-zinc-300 bg-zinc-800/60 border border-white/5 px-2 py-0.5 rounded font-mono">undefined</code> sandbox directory.
                  </p>

                  {/* Unified Visual Input Weight Fields */}
                  <div className="space-y-2">
                    <div className="flex gap-3">
                      <div className="relative flex-1">
                        <Input
                          value={newCatInput}
                          onChange={(e) => {
                            setNewCatInput(e.target.value);
                            setNewCatError(null);
                          }}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") {
                              e.preventDefault();
                              handleAddCategory();
                            }
                          }}
                          className="text-sm border-white/5 bg-black/40 h-10 focus-visible:ring-1 focus-visible:ring-indigo-500/50"
                          placeholder="Type new category (e.g. sandbox, playground)..."
                        />
                        {newCatInput && (
                          <button
                            onClick={() => {
                              setNewCatInput("");
                              setNewCatError(null);
                            }}
                            className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-white transition-colors"
                          >
                            <X className="size-4" />
                          </button>
                        )}
                      </div>
                      <Button
                        onClick={handleAddCategory}
                        disabled={!newCatInput.trim() || savingCategories}
                        className="bg-indigo-600 hover:bg-indigo-500 text-white font-semibold h-10 px-5 transition-all duration-150 active:scale-95 disabled:opacity-50"
                      >
                        <Plus className="size-4 mr-1.5" />
                        Add Folder
                      </Button>
                    </div>
                    {newCatError && (
                      <p className="text-xs text-rose-400 flex items-center gap-1.5 animate-fadeIn">
                        <AlertCircle className="size-3.5" />
                        {newCatError}
                      </p>
                    )}
                  </div>

                  {/* Separator and Header for Active Folders Structure */}
                  <div className="mt-8 pt-6 border-t border-white/5 space-y-4">
                    <div className="flex items-center justify-between">
                      <label className="text-xs font-bold text-zinc-400 uppercase tracking-widest block">
                        Active Folders Structure
                      </label>
                      <span className="text-[10px] text-zinc-500 font-mono">
                        Total: {categories.length} folders
                      </span>
                    </div>

                    {/* Viewport Categories Grid with Fade indicator and scrollbar */}
                    {categories.length === 0 ? (
                      <div className="flex flex-col items-center justify-center p-8 bg-zinc-900/20 border border-dashed border-white/5 rounded-xl">
                        <ShieldAlert className="size-8 text-amber-500/60 mb-2" />
                        <p className="text-sm text-zinc-400">No active categories configured.</p>
                        <p className="text-xs text-muted-foreground mt-1">Add one above to enable dynamic organization.</p>
                      </div>
                    ) : (
                      <div className="max-h-[280px] overflow-y-auto pr-1 space-y-3.5 scrollbar-thin scrollbar-thumb-zinc-800 scrollbar-track-transparent">
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-3.5 pb-2">
                          {categories.map((cat, idx) => {
                            const style = getCategoryStyle(cat, idx);
                            return (
                              <div
                                key={cat}
                                className={`group flex flex-col justify-between p-4 ${style.bg} border ${style.border} rounded-xl transition-all duration-200`}
                              >
                                <div className="flex items-center justify-between">
                                  <div className="flex items-center gap-2">
                                    <span className={`w-1.5 h-1.5 rounded-full ${style.dot} animate-pulse`} />
                                    <span className={`font-mono text-sm font-semibold ${style.text} capitalize`}>
                                      {cat}
                                    </span>
                                  </div>
                                  <button
                                    onClick={() => handleRemoveCategory(cat)}
                                    className="text-zinc-500 hover:text-rose-400 p-2 rounded-md hover:bg-rose-500/10 transition-all duration-150 opacity-80 hover:opacity-100"
                                    title={`Remove ${cat} category`}
                                  >
                                    <Trash2 className="size-3.5" />
                                  </button>
                                </div>
                                
                                <div className="mt-3 text-[10px] font-mono text-zinc-500 truncate select-all leading-normal">
                                  Path: {baseDir || "~/projects"}/{cat}
                                </div>
                              </div>
                            );
                          })}
                        </div>
                      </div>
                    )}
                  </div>

                  {/* Cohesive Visual Color Coding Legend */}
                  <div className="pt-4 border-t border-white/5 space-y-2">
                    <label className="text-[10px] font-bold text-zinc-500 uppercase tracking-widest block">
                      Category Color Code Legend
                    </label>
                    <div className="flex flex-wrap gap-x-4 gap-y-2 text-[10px] text-zinc-400">
                      <div className="flex items-center gap-1.5">
                        <span className="w-1.5 h-1.5 rounded-full bg-indigo-500" />
                        <span>Apps (Indigo)</span>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <span className="w-1.5 h-1.5 rounded-full bg-cyan-500" />
                        <span>Services (Cyan)</span>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <span className="w-1.5 h-1.5 rounded-full bg-fuchsia-500" />
                        <span>UI/Web (Fuchsia)</span>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <span className="w-1.5 h-1.5 rounded-full bg-amber-500" />
                        <span>Embedded (Amber)</span>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                        <span>ML/AI (Emerald)</span>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <span className="w-1.5 h-1.5 rounded-full bg-zinc-400" />
                        <span>Tools (Zinc)</span>
                      </div>
                    </div>
                  </div>

                  {/* Save feedback indicator */}
                  {categoryMessage && (
                    <div className={`p-3 rounded-lg border text-sm flex items-center gap-2 ${
                      categoryMessage.ok 
                        ? "bg-emerald-500/10 text-emerald-400 border-emerald-500/20" 
                        : "bg-rose-500/10 text-rose-400 border-rose-500/20"
                    }`}>
                      {categoryMessage.ok ? (
                        <Check className="size-4.5 shrink-0" />
                      ) : (
                        <AlertCircle className="size-4.5 shrink-0" />
                      )}
                      {categoryMessage.text}
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>
          )}

          {/* TAB 3: BLUEPRINTS (PROJECT CREATION TEMPLATES) */}
          {activeTab === "blueprints" && (
            <div className="space-y-6 animate-in fade-in duration-200">
              <Card className="border border-white/5 bg-zinc-950/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden transition-all duration-300 hover:border-white/10">
                <CardHeader className="p-6 pb-4">
                  <CardTitle className="text-xs font-bold tracking-widest uppercase text-indigo-400 flex items-center gap-2.5">
                    <Sparkles className="size-4.5 text-indigo-400" />
                    Project Blueprints
                  </CardTitle>
                </CardHeader>
                <CardContent className="p-6 pt-0 space-y-5">
                  <p className="text-sm text-zinc-400 leading-relaxed">
                    Save command templates for scaffolding new projects. Use{" "}
                    <code className="text-xs bg-zinc-800/60 text-zinc-300 px-1.5 py-0.5 rounded border border-white/5 font-mono">{`{name}`}</code>{" "}
                    as an optional placeholder — you will be prompted for a project name when running.
                  </p>

                  {/* Add Blueprint Form */}
                  <div className="p-4 bg-zinc-900/20 border border-white/5 rounded-xl space-y-3.5">
                    <label className="text-[10px] font-bold text-zinc-500 uppercase tracking-widest">
                      New Blueprint
                    </label>
                    <div className="space-y-3">
                      <Input
                        value={newBlueprintName}
                        onChange={(e) => { setNewBlueprintName(e.target.value); setBlueprintError(null); }}
                        placeholder="Blueprint name (e.g. better-t-stack)"
                        className="text-sm bg-black/40 border-white/5 h-10 focus-visible:ring-1 focus-visible:ring-indigo-500/50"
                      />
                      <Input
                        value={newBlueprintCommand}
                        onChange={(e) => { setNewBlueprintCommand(e.target.value); setBlueprintError(null); }}
                        placeholder={`Command template (e.g. npx create-t3-app {name})`}
                        className="text-sm bg-black/40 border-white/5 h-10 font-mono focus-visible:ring-1 focus-visible:ring-indigo-500/50"
                      />
                      {blueprintError && (
                        <p className="text-xs text-rose-400 flex items-center gap-1.5">
                          <AlertCircle className="size-3.5" />
                          {blueprintError}
                        </p>
                      )}
                      <div className="flex justify-end">
                        <Button
                          onClick={handleAddBlueprint}
                          disabled={!newBlueprintName.trim() || !newBlueprintCommand.trim() || savingBlueprint}
                          className="bg-indigo-600 hover:bg-indigo-500 text-white font-semibold h-9 px-4 text-xs transition-all active:scale-95 disabled:opacity-50"
                        >
                          <Plus className="size-3.5 mr-1.5" />
                          {savingBlueprint ? "Adding..." : "Add Blueprint"}
                        </Button>
                      </div>
                    </div>
                  </div>

                  {/* Blueprints List */}
                  <div className="space-y-3 max-h-[420px] overflow-y-auto pr-1 scrollbar-thin scrollbar-thumb-zinc-800 scrollbar-track-transparent">
                    {blueprints.length === 0 ? (
                      <div className="flex flex-col items-center justify-center p-8 bg-zinc-900/10 border border-dashed border-white/5 rounded-xl">
                        <BookOpen className="size-8 text-zinc-600 mb-2" />
                        <p className="text-xs text-zinc-400">No project blueprints saved yet.</p>
                        <p className="text-[10px] text-zinc-500 mt-1">Add one above to get started.</p>
                      </div>
                    ) : (
                      blueprints.map((bp, idx) => {
                        const isEditing = editingBlueprint === bp.name;
                        return (
                          <div
                            key={bp.name}
                            className="p-4 bg-zinc-900/20 border border-white/5 hover:border-white/10 rounded-xl space-y-3 transition-all"
                          >
                            {isEditing ? (
                              /* Edit Mode */
                              <div className="space-y-3">
                                <Input
                                  value={editBlueprintName}
                                  onChange={(e) => setEditBlueprintName(e.target.value)}
                                  placeholder="Blueprint name"
                                  className="h-8 text-xs bg-black/40 border-white/5 focus-visible:ring-1 focus-visible:ring-indigo-500/50"
                                />
                                <Input
                                  value={editBlueprintCommand}
                                  onChange={(e) => setEditBlueprintCommand(e.target.value)}
                                  placeholder="Command template"
                                  className="h-8 text-xs bg-black/40 border-white/5 font-mono focus-visible:ring-1 focus-visible:ring-indigo-500/50"
                                />
                                <div className="flex justify-end gap-2">
                                  <Button
                                    onClick={() => setEditingBlueprint(null)}
                                    variant="ghost"
                                    className="text-xs text-zinc-400 hover:text-white h-8 px-3"
                                  >
                                    Cancel
                                  </Button>
                                  <Button
                                    onClick={() => handleSaveEditBlueprint(bp.name)}
                                    disabled={!editBlueprintName.trim() || !editBlueprintCommand.trim()}
                                    className="bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-semibold h-8 px-3 transition-all active:scale-95"
                                  >
                                    <Save className="size-3 mr-1.5" />
                                    Save
                                  </Button>
                                </div>
                              </div>
                            ) : (
                              /* View Mode */
                              <>
                                <div className="flex items-center justify-between">
                                  <div className="flex items-center gap-2">
                                    <span className={`w-1.5 h-1.5 rounded-full ${idx % 2 === 0 ? "bg-indigo-500" : "bg-cyan-500"} shadow-[0_0_6px_rgba(99,102,241,0.5)]`} />
                                    <span className="font-mono text-sm font-semibold text-zinc-200">{bp.name}</span>
                                  </div>
                                  <div className="flex items-center gap-1">
                                    <button
                                      onClick={() => startEditBlueprint(bp)}
                                      className="p-1.5 text-zinc-500 hover:text-white hover:bg-white/5 rounded transition-all"
                                      title="Edit blueprint"
                                    >
                                      <svg className="size-3.5" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor">
                                        <path strokeLinecap="round" strokeLinejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0115.75 21H5.25A2.25 2.25 0 013 18.75V8.25A2.25 2.25 0 015.25 6H10" />
                                      </svg>
                                    </button>
                                    <button
                                      onClick={() => handleDeleteBlueprint(bp.name)}
                                      className="p-1.5 text-zinc-500 hover:text-rose-400 hover:bg-rose-500/10 rounded transition-all"
                                      title="Delete blueprint"
                                    >
                                      <Trash2 className="size-3.5" />
                                    </button>
                                  </div>
                                </div>
                                <div className="text-[11px] font-mono text-zinc-500 bg-black/30 px-3 py-2 rounded-lg border border-white/5 truncate select-all">
                                  {bp.command}
                                </div>
                              </>
                            )}
                          </div>
                        );
                      })
                    )}
                  </div>

                  {blueprintMessage && (
                    <div className={`p-3 rounded-lg border text-xs flex items-center gap-2 ${
                      blueprintMessage.ok
                        ? "bg-emerald-500/10 text-emerald-400 border-emerald-500/20"
                        : "bg-rose-500/10 text-rose-400 border-rose-500/20"
                    }`}>
                      {blueprintMessage.ok ? (
                        <Check className="size-4 shrink-0" />
                      ) : (
                        <AlertCircle className="size-4 shrink-0" />
                      )}
                      {blueprintMessage.text}
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>
          )}

          {/* TAB: AI AGENTS (CODING AGENT CLIS) */}
          {activeTab === "agents" && (
            <div className="space-y-6 animate-in fade-in duration-200">
              <Card className="border border-white/5 bg-zinc-950/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden transition-all duration-300 hover:border-white/10">
                <CardHeader className="p-6 pb-4">
                  <CardTitle className="text-xs font-bold tracking-widest uppercase text-emerald-400 flex items-center gap-2.5">
                    <Bot className="size-4.5 text-emerald-400" />
                    AI Coding Agents
                  </CardTitle>
                </CardHeader>
                <CardContent className="p-6 pt-0 space-y-5">
                  <p className="text-sm text-zinc-400 leading-relaxed">
                    Configure the AI agent CLIs you can launch inside any project&apos;s shell —
                    Claude Code, Codex, Gemini CLI, OpenCode, Aider, or any custom command.
                    Launch them from a project&apos;s <span className="text-zinc-300 font-semibold">Run &amp; Shell</span> panel
                    via the <span className="text-zinc-300 font-semibold">AI Agent</span> button.
                  </p>

                  {/* Add Agent Form */}
                  <div className="p-4 bg-zinc-900/20 border border-white/5 rounded-xl space-y-3.5">
                    <label className="text-[10px] font-bold text-zinc-500 uppercase tracking-widest">
                      New Agent
                    </label>
                    <div className="space-y-3">
                      <Input
                        value={newAgentName}
                        onChange={(e) => { setNewAgentName(e.target.value); setAgentError(null); }}
                        placeholder="Agent name (e.g. Claude Code)"
                        className="text-sm bg-black/40 border-white/5 h-10 focus-visible:ring-1 focus-visible:ring-emerald-500/50"
                      />
                      <Input
                        value={newAgentCommand}
                        onChange={(e) => { setNewAgentCommand(e.target.value); setAgentError(null); }}
                        placeholder="Launch command (e.g. claude, codex --model gpt-5, aider)"
                        className="text-sm bg-black/40 border-white/5 h-10 font-mono focus-visible:ring-1 focus-visible:ring-emerald-500/50"
                      />
                      {agentError && (
                        <p className="text-xs text-rose-400 flex items-center gap-1.5">
                          <AlertCircle className="size-3.5" />
                          {agentError}
                        </p>
                      )}
                      <div className="flex justify-end">
                        <Button
                          onClick={handleAddAgent}
                          disabled={!newAgentName.trim() || !newAgentCommand.trim() || savingAgent}
                          className="bg-emerald-600 hover:bg-emerald-500 text-white font-semibold h-9 px-4 text-xs transition-all active:scale-95 disabled:opacity-50"
                        >
                          <Plus className="size-3.5 mr-1.5" />
                          {savingAgent ? "Adding..." : "Add Agent"}
                        </Button>
                      </div>
                    </div>
                  </div>

                  {/* Agents List */}
                  <div className="space-y-3 max-h-[420px] overflow-y-auto pr-1 scrollbar-thin scrollbar-thumb-zinc-800 scrollbar-track-transparent">
                    {agents.length === 0 ? (
                      <div className="flex flex-col items-center justify-center p-8 bg-zinc-900/10 border border-dashed border-white/5 rounded-xl">
                        <Bot className="size-8 text-zinc-600 mb-2" />
                        <p className="text-xs text-zinc-400">No AI agents configured yet.</p>
                        <p className="text-[10px] text-zinc-500 mt-1">Add one above to get started.</p>
                      </div>
                    ) : (
                      agents.map((agent) => {
                        const isEditing = editingAgent === agent.name;
                        return (
                          <div
                            key={agent.name}
                            className="p-4 bg-zinc-900/20 border border-white/5 hover:border-white/10 rounded-xl space-y-3 transition-all"
                          >
                            {isEditing ? (
                              /* Edit Mode */
                              <div className="space-y-3">
                                <Input
                                  value={editAgentName}
                                  onChange={(e) => setEditAgentName(e.target.value)}
                                  placeholder="Agent name"
                                  className="h-8 text-xs bg-black/40 border-white/5 focus-visible:ring-1 focus-visible:ring-emerald-500/50"
                                />
                                <Input
                                  value={editAgentCommand}
                                  onChange={(e) => setEditAgentCommand(e.target.value)}
                                  placeholder="Launch command"
                                  className="h-8 text-xs bg-black/40 border-white/5 font-mono focus-visible:ring-1 focus-visible:ring-emerald-500/50"
                                />
                                <div className="flex justify-end gap-2">
                                  <Button
                                    onClick={() => setEditingAgent(null)}
                                    variant="ghost"
                                    className="text-xs text-zinc-400 hover:text-white h-8 px-3"
                                  >
                                    Cancel
                                  </Button>
                                  <Button
                                    onClick={() => handleSaveEditAgent(agent.name)}
                                    disabled={!editAgentName.trim() || !editAgentCommand.trim()}
                                    className="bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold h-8 px-3 transition-all active:scale-95"
                                  >
                                    <Save className="size-3 mr-1.5" />
                                    Save
                                  </Button>
                                </div>
                              </div>
                            ) : (
                              /* View Mode */
                              <>
                                <div className="flex items-center justify-between">
                                  <div className="flex items-center gap-2.5">
                                    <span className={`w-1.5 h-1.5 rounded-full ${agent.installed ? "bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.5)]" : "bg-zinc-600"}`} />
                                    <span className="font-mono text-sm font-semibold text-zinc-200">{agent.name}</span>
                                    {agent.installed ? (
                                      <Badge className="bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-[9px] font-mono py-0 px-1.5">
                                        <Check className="size-2.5 mr-1" />
                                        installed
                                      </Badge>
                                    ) : (
                                      <Badge className="bg-zinc-800/60 text-zinc-500 border border-white/5 text-[9px] font-mono py-0 px-1.5">
                                        not found on $PATH
                                      </Badge>
                                    )}
                                  </div>
                                  <div className="flex items-center gap-1">
                                    <button
                                      onClick={() => startEditAgent(agent)}
                                      className="p-1.5 text-zinc-500 hover:text-white hover:bg-white/5 rounded transition-all"
                                      title="Edit agent"
                                    >
                                      <svg className="size-3.5" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor">
                                        <path strokeLinecap="round" strokeLinejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0115.75 21H5.25A2.25 2.25 0 013 18.75V8.25A2.25 2.25 0 015.25 6H10" />
                                      </svg>
                                    </button>
                                    <button
                                      onClick={() => handleDeleteAgent(agent.name)}
                                      className="p-1.5 text-zinc-500 hover:text-rose-400 hover:bg-rose-500/10 rounded transition-all"
                                      title="Delete agent"
                                    >
                                      <Trash2 className="size-3.5" />
                                    </button>
                                  </div>
                                </div>
                                <div className="flex items-center gap-2 text-[11px] font-mono text-zinc-500 bg-black/30 px-3 py-2 rounded-lg border border-white/5">
                                  <Terminal className="size-3 shrink-0 text-zinc-600" />
                                  <span className="truncate select-all">{agent.command}</span>
                                </div>
                                {agent.installed && agent.path && (
                                  <p className="text-[10px] font-mono text-zinc-600 truncate" title={agent.path}>
                                    {agent.path}
                                  </p>
                                )}
                              </>
                            )}
                          </div>
                        );
                      })
                    )}
                  </div>

                  {agentMessage && (
                    <div className={`p-3 rounded-lg border text-xs flex items-center gap-2 ${
                      agentMessage.ok
                        ? "bg-emerald-500/10 text-emerald-400 border-emerald-500/20"
                        : "bg-rose-500/10 text-rose-400 border-rose-500/20"
                    }`}>
                      {agentMessage.ok ? (
                        <Check className="size-4 shrink-0" />
                      ) : (
                        <AlertCircle className="size-4 shrink-0" />
                      )}
                      {agentMessage.text}
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>
          )}

          {/* TAB 4: RULES CONFIG (FORMERLY BLUEPRINT RULES) */}
          {activeTab === "rules" && (
            <div className="space-y-6 animate-in fade-in duration-200">
              <Card className="border border-white/5 bg-zinc-950/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden transition-all duration-300 hover:border-white/10">
                <CardHeader className="p-6 pb-4 border-b border-white/5 bg-zinc-950/20 flex flex-row items-center justify-between">
                  <div className="space-y-1">
                    <CardTitle className="text-xs font-bold tracking-widest uppercase text-indigo-400 flex items-center gap-2.5">
                      <BookOpen className="size-4.5 text-indigo-400" />
                      Classification Rules Config
                    </CardTitle>
                    <p className="text-[10px] text-zinc-500 lowercase leading-none">
                      Custom classification rules — evaluated top to bottom inside ~/.config/projm/rules.toml
                    </p>
                  </div>
                </CardHeader>
                <CardContent className="p-6 space-y-6">
                  {rulesError && (
                    <div className="p-3.5 bg-rose-500/10 text-rose-400 border border-rose-500/20 rounded-xl text-xs flex items-start gap-2 animate-in slide-in-from-top-2">
                      <AlertCircle className="size-4.5 shrink-0 mt-0.5" />
                      <div className="space-y-0.5">
                        <span className="font-semibold">Configuration Error</span>
                        <p className="font-mono text-[10px] leading-relaxed">{rulesError}</p>
                      </div>
                    </div>
                  )}

                  {/* VISUAL DESIGNER MODE */}
                  {isVisualMode ? (
                    <div className="space-y-4">
                      <p className="text-xs text-zinc-400 leading-relaxed">
                        Customize custom priority path mappings. The organizer runs these rules sequentially, placing matched directories into your dynamic category folders immediately.
                      </p>

                      <div className="space-y-4 max-h-[380px] overflow-y-auto pr-1 scrollbar-thin scrollbar-thumb-zinc-800 scrollbar-track-transparent py-1">
                        {rulesList.length === 0 ? (
                          <div className="flex flex-col items-center justify-center p-8 bg-zinc-900/10 border border-dashed border-white/5 rounded-xl">
                            <BookOpen className="size-8 text-zinc-600 mb-2" />
                            <p className="text-xs text-zinc-400">No custom priority rules set up yet.</p>
                            <p className="text-[10px] text-zinc-500 mt-1">Create one below to establish overrides.</p>
                          </div>
                        ) : (
                          rulesList.map((rule, idx) => (
                            <div
                              key={idx}
                              className="p-4 bg-zinc-900/20 border border-white/5 hover:border-white/10 rounded-xl space-y-3.5 transition-all"
                            >
                              {/* Rule Card Header */}
                              <div className="flex items-center justify-between">
                                <div className="flex items-center gap-2">
                                  <Badge className="bg-zinc-800/80 text-zinc-400 font-mono text-[9px] border border-white/5">
                                    Priority #{idx + 1}
                                  </Badge>
                                  <span className="text-[10px] text-zinc-500 italic">
                                    (Evaluates {idx === 0 ? "first" : "next"})
                                  </span>
                                </div>

                                <div className="flex items-center gap-1.5">
                                  {/* Priority Reordering Buttons */}
                                  <button
                                    onClick={() => handleMoveRule(idx, "up")}
                                    disabled={idx === 0}
                                    className="p-1 text-zinc-500 hover:text-white hover:bg-white/5 rounded disabled:opacity-30 disabled:pointer-events-none"
                                    title="Increase Priority (Move Up)"
                                  >
                                    <ChevronUp className="size-4" />
                                  </button>
                                  <button
                                    onClick={() => handleMoveRule(idx, "down")}
                                    disabled={idx === rulesList.length - 1}
                                    className="p-1 text-zinc-500 hover:text-white hover:bg-white/5 rounded disabled:opacity-30 disabled:pointer-events-none"
                                    title="Decrease Priority (Move Down)"
                                  >
                                    <ChevronDown className="size-4" />
                                  </button>
                                  <Separator orientation="vertical" className="h-4 bg-white/5 mx-1" />
                                  <button
                                    onClick={() => handleRemoveRule(idx)}
                                    className="p-1 text-zinc-500 hover:text-rose-400 hover:bg-rose-500/10 rounded"
                                    title="Delete custom rule"
                                  >
                                    <Trash2 className="size-4" />
                                  </button>
                                </div>
                              </div>

                              {/* Rule Matching Fields */}
                              <div className="grid grid-cols-1 md:grid-cols-2 gap-3.5">
                                <div className="space-y-1">
                                  <label className="text-[9px] font-bold text-zinc-500 uppercase tracking-widest">
                                    Folder Name Matches Exactly
                                  </label>
                                  <Input
                                    value={rule.name || ""}
                                    onChange={(e) => handleUpdateRuleField(idx, "name", e.target.value)}
                                    placeholder="e.g. pioneers-website"
                                    className="h-8 text-xs bg-black/40 border-white/5 focus-visible:ring-1 focus-visible:ring-indigo-500/50 font-mono"
                                  />
                                </div>
                                <div className="space-y-1">
                                  <label className="text-[9px] font-bold text-zinc-500 uppercase tracking-widest">
                                    Folder Name Contains
                                  </label>
                                  <Input
                                    value={rule.name_contains || ""}
                                    onChange={(e) => handleUpdateRuleField(idx, "name_contains", e.target.value)}
                                    placeholder="e.g. frontend"
                                    className="h-8 text-xs bg-black/40 border-white/5 focus-visible:ring-1 focus-visible:ring-indigo-500/50 font-mono"
                                  />
                                </div>
                                <div className="space-y-1">
                                  <label className="text-[9px] font-bold text-zinc-500 uppercase tracking-widest">
                                    File Marker Exists in Root
                                  </label>
                                  <Input
                                    value={rule.marker || ""}
                                    onChange={(e) => handleUpdateRuleField(idx, "marker", e.target.value)}
                                    placeholder="e.g. package.json, rocket.toml"
                                    className="h-8 text-xs bg-black/40 border-white/5 focus-visible:ring-1 focus-visible:ring-indigo-500/50 font-mono"
                                  />
                                </div>
                                <div className="space-y-1">
                                  <label className="text-[9px] font-bold text-zinc-500 uppercase tracking-widest">
                                    Suffix Override Tag
                                  </label>
                                  <Input
                                    value={rule.suffix || ""}
                                    onChange={(e) => handleUpdateRuleField(idx, "suffix", e.target.value)}
                                    placeholder="e.g. ui, core, backend"
                                    className="h-8 text-xs bg-black/40 border-white/5 focus-visible:ring-1 focus-visible:ring-indigo-500/50 font-mono"
                                  />
                                </div>
                                <div className="space-y-1">
                                  <label className="text-[9px] font-bold text-zinc-500 uppercase tracking-widest">
                                    Has Package Dependency
                                  </label>
                                  <Input
                                    value={rule.has_dep || ""}
                                    onChange={(e) => handleUpdateRuleField(idx, "has_dep", e.target.value)}
                                    placeholder="e.g. react, tensorflow, burn"
                                    className="h-8 text-xs bg-black/40 border-white/5 focus-visible:ring-1 focus-visible:ring-indigo-500/50 font-mono"
                                  />
                                </div>
                                <div className="space-y-1">
                                  <label className="text-[9px] font-bold text-zinc-500 uppercase tracking-widest">
                                    Destination Category Folder
                                  </label>
                                  <select
                                    value={rule.category}
                                    onChange={(e) => handleUpdateRuleField(idx, "category", e.target.value)}
                                    className="w-full h-8 px-2.5 rounded-md text-xs font-semibold bg-black/40 border border-white/5 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-indigo-500/50 text-indigo-300 capitalize font-mono"
                                  >
                                    {categories.map((c) => (
                                      <option key={c} value={c} className="bg-zinc-950 text-indigo-400 capitalize">
                                        {c}
                                      </option>
                                    ))}
                                    <option value="undefined" className="bg-zinc-950 text-rose-400 italic">
                                      undefined
                                    </option>
                                  </select>
                                </div>
                              </div>
                            </div>
                          ))
                        )}
                      </div>

                      <div className="flex justify-between items-center pt-2">
                        <Button
                          onClick={handleAddBlankRule}
                          variant="ghost"
                          className="text-xs text-indigo-400 hover:text-white hover:bg-white/5 border border-white/5 border-dashed rounded-lg"
                        >
                          <Plus className="size-3.5 mr-1" />
                          Add Custom Rule
                        </Button>

                        <Button
                          onClick={handleSaveBlueprintRules}
                          disabled={savingRules}
                          className="bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-semibold h-9 px-4 active:scale-95 transition-all"
                        >
                          {savingRules ? (
                            "Saving Rules..."
                          ) : (
                            <>
                              <Save className="size-3.5 mr-1.5" />
                              Save Rules Configuration
                            </>
                          )}
                        </Button>
                      </div>
                    </div>
                  ) : (
                    /* RAW TOML CODE MODE */
                    <div className="space-y-4 animate-in fade-in duration-150">
                      <p className="text-xs text-zinc-400 leading-relaxed">
                        Edit raw specifications directly. If you modify options below, clicking Save triggers internal Rust validation checks to guarantee syntax compliance.
                      </p>

                      <div className="relative">
                        <textarea
                          value={rulesRaw}
                          onChange={(e) => {
                            setRulesRaw(e.target.value);
                            setRulesError(null);
                          }}
                          spellCheck={false}
                          className="w-full h-[320px] bg-black/50 border border-white/5 p-4 rounded-xl font-mono text-xs text-zinc-300 focus:outline-none focus:ring-1 focus:ring-indigo-500/50 leading-relaxed scrollbar-thin scrollbar-thumb-zinc-800 scrollbar-track-transparent select-text"
                          placeholder="# Add your custom rules config here..."
                        />
                      </div>

                      <div className="flex justify-end pt-2">
                        <Button
                          onClick={handleSaveBlueprintRules}
                          disabled={savingRules || !rulesRaw.trim()}
                          className="bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-semibold h-9 px-4 active:scale-95 transition-all"
                        >
                          {savingRules ? (
                            "Validating & Saving..."
                          ) : (
                            <>
                              <Save className="size-3.5 mr-1.5" />
                              Save TOML Code
                            </>
                          )}
                        </Button>
                      </div>
                    </div>
                  )}

                  {rulesMessage && (
                    <div className={`p-3 rounded-lg border text-xs flex items-center gap-2 animate-in slide-in-from-bottom-2 ${
                      rulesMessage.ok 
                        ? "bg-emerald-500/10 text-emerald-400 border-emerald-500/20" 
                        : "bg-rose-500/10 text-rose-400 border-rose-500/20"
                    }`}>
                      {rulesMessage.ok ? (
                        <Check className="size-4 shrink-0" />
                      ) : (
                        <AlertCircle className="size-4 shrink-0" />
                      )}
                      {rulesMessage.text}
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>
          )}

          {/* TAB 4: ENGINE SPECS */}
          {activeTab === "engine" && (
            <div className="space-y-6 animate-in fade-in duration-200">
              {/* About Engine Card */}
              <Card className="border border-white/5 bg-zinc-950/40 backdrop-blur-md rounded-xl shadow-none overflow-hidden transition-all duration-300 hover:border-white/10">
                <CardHeader className="p-6 pb-4 border-b border-white/5 bg-zinc-950/20">
                  <CardTitle className="text-xs font-bold tracking-widest uppercase text-slate-400 flex items-center gap-2.5">
                    <Info className="size-4.5 text-slate-400" />
                    Engine Specifications
                  </CardTitle>
                </CardHeader>
                <CardContent className="p-6 space-y-3.5 text-xs">
                  <div className="flex items-center justify-between">
                    <span className="text-zinc-500">Core Service Identifier</span>
                    <span className="font-semibold text-zinc-300 font-mono">Projm Organizer</span>
                  </div>
                  <Separator className="bg-white/5" />
                  <div className="flex items-center justify-between">
                    <span className="text-zinc-500">Active Build Version</span>
                    <span className="font-mono text-xs text-indigo-400 bg-indigo-500/10 px-2 py-0.5 rounded border border-indigo-500/20">
                      0.7.1
                    </span>
                  </div>
                  <Separator className="bg-white/5" />
                  <div className="flex items-center justify-between">
                    <span className="text-zinc-500">Engine Description</span>
                    <span className="text-right text-[11px] text-zinc-400 max-w-[220px] leading-relaxed">
                      Automated classification system and jump navigator for local developer workspaces
                    </span>
                  </div>
                  <Separator className="bg-white/5" />
                  <div className="flex items-center justify-between">
                    <span className="text-zinc-500">Global Configuration File</span>
                    <code className="text-[10px] bg-zinc-900/60 text-zinc-300 px-2 py-0.5 rounded border border-white/5 font-mono truncate max-w-[200px]">
                      ~/.config/projm/config.json
                    </code>
                  </div>
                </CardContent>
              </Card>
            </div>
          )}

        </div>

        {/* Right Column - Tree Preview Simulator (Spans 5 columns, Persistent across all tabs) */}
        <div className="lg:col-span-5 h-full overflow-hidden flex flex-col">
          <DirectoryTree baseDir={baseDir} categories={categories} />
        </div>

      </div>

      {/* Warning Deletion Modal (Slide up spring transitions) */}
      {categoryToDelete && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm animate-in fade-in zoom-in-95 duration-200">
          <div className="w-full max-w-md p-6 bg-zinc-950 border border-white/10 rounded-2xl shadow-xl space-y-4">
            <div className="flex items-start gap-3.5">
              <div className="p-2.5 bg-rose-500/10 border border-rose-500/20 rounded-xl">
                <ShieldAlert className="size-6 text-rose-400" />
              </div>
              <div className="space-y-1">
                <h3 className="text-lg font-bold text-zinc-200">
                  Delete Folder Category?
                </h3>
                <p className="text-sm text-zinc-400 leading-relaxed">
                  Are you sure you want to remove the <code className="text-xs text-rose-300 bg-rose-500/10 px-1.5 py-0.5 rounded font-mono">"{categoryToDelete}"</code> category folder?
                </p>
              </div>
            </div>

            <p className="text-xs text-rose-400/90 leading-relaxed bg-rose-500/5 p-3.5 rounded-lg border border-rose-500/10">
              <strong>Important</strong>: Deactivating this folder means any active projects matching this category will be dynamically coerced and moved to the fallback <code className="text-xs bg-rose-500/20 font-bold px-1 rounded text-white">undefined</code> folder during the next reorganization scan.
            </p>

            <div className="flex justify-end gap-2.5 pt-2">
              <Button
                onClick={() => setCategoryToDelete(null)}
                variant="ghost"
                className="text-zinc-400 hover:text-white font-medium hover:bg-white/5 transition-all"
              >
                Cancel
              </Button>
              <Button
                onClick={confirmDeleteCategory}
                className="bg-rose-600 hover:bg-rose-500 text-white font-semibold transition-all active:scale-95"
              >
                Deactivate Folder
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
