"use client";

import { X } from "lucide-react";

interface TabProject {
  name: String;
  path: String;
  category: string;
}

type AppStatuses = Record<string, string>;

function sessionDot(statuses: AppStatuses | undefined): string {
  const values = Object.values(statuses ?? {});
  if (values.includes("running")) return "bg-emerald-400";
  if (values.includes("starting")) return "bg-amber-400 animate-pulse";
  if (values.includes("errored")) return "bg-red-500";
  return "bg-slate-600";
}

/**
 * Open-project tab strip for the workspace top header. One tab per opened
 * project; the dot mirrors that project's runner session activity. Click
 * selects, × closes (without selecting).
 */
export default function ProjectTabs({
  projects,
  activePath,
  activity,
  onSelect,
  onClose,
}: {
  projects: TabProject[];
  activePath: string | null;
  activity: Record<string, AppStatuses>;
  onSelect: (p: TabProject) => void;
  onClose: (path: string) => void;
}) {
  return (
    <div
      role="tablist"
      className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto scrollbar-none"
    >
      {projects.map((p) => {
        const path = p.path.toString();
        const isActive = path === activePath;
        return (
          <div
            key={path}
            role="tab"
            aria-selected={isActive}
            tabIndex={0}
            onClick={() => onSelect(p)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") onSelect(p);
            }}
            title={`~/projects/${p.category}/${p.name}`}
            className={`group flex h-7 shrink-0 cursor-pointer items-center gap-1.5 rounded-md border px-2.5 text-xs font-mono transition-colors ${
              isActive
                ? "bg-[#18191c] border-indigo-500/40 text-indigo-200"
                : "border-transparent text-[#94a3b8] hover:bg-[#181a1c]/60 hover:text-white"
            }`}
          >
            <span
              className={`h-1.5 w-1.5 shrink-0 rounded-full ${sessionDot(activity[path])}`}
            />
            <span className="max-w-[130px] truncate">{p.name}</span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onClose(path);
              }}
              title={`Close ${p.name}`}
              className={`rounded p-0.5 text-[#64748b] transition-opacity hover:bg-white/10 hover:text-white ${
                isActive ? "opacity-100" : "opacity-0 group-hover:opacity-100"
              }`}
            >
              <X className="size-3" />
            </button>
          </div>
        );
      })}
    </div>
  );
}
