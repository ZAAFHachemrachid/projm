"use client";

import { useEffect, useRef, useState } from "react";
import { Check, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";

export interface DropdownOption {
  value: string;
  label: string;
  /** Trailing dim text, e.g. "— not found" or "(→ /bin/zsh)". */
  hint?: string;
  /** Per-option classes, e.g. "capitalize text-primary". */
  className?: string;
}

interface DropdownProps {
  value: string;
  onChange: (value: string) => void;
  options: DropdownOption[];
  /** Classes for the positioning wrapper (e.g. "flex-1", "w-full"). */
  className?: string;
  /** Classes for the trigger button (sizing/typography). */
  triggerClassName?: string;
  placeholder?: string;
  mono?: boolean;
  disabled?: boolean;
}

/**
 * Our own dropdown — a fully-styled, token-driven replacement for the native
 * <select>. Closes on outside-click / Escape, supports keyboard navigation,
 * and themes with the rest of the app.
 */
export function Dropdown({
  value,
  onChange,
  options,
  className,
  triggerClassName,
  placeholder = "Select…",
  mono,
  disabled,
}: DropdownProps) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const rootRef = useRef<HTMLDivElement>(null);
  const selected = options.find((o) => o.value === value);

  // Close on outside click.
  useEffect(() => {
    if (!open) return;
    function onDoc(e: MouseEvent) {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  // Open and highlight the current value in one step (no setState-in-effect).
  function openMenu() {
    setActive(options.findIndex((o) => o.value === value));
    setOpen(true);
  }

  function choose(v: string) {
    onChange(v);
    setOpen(false);
  }

  function onKeyDown(e: React.KeyboardEvent) {
    if (disabled) return;
    if (!open) {
      if (e.key === "Enter" || e.key === " " || e.key === "ArrowDown") {
        e.preventDefault();
        openMenu();
      }
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      setOpen(false);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((i) => Math.min(options.length - 1, i + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((i) => Math.max(0, i - 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (options[active]) choose(options[active].value);
    }
  }

  return (
    <div ref={rootRef} className={cn("relative", className)}>
      <button
        type="button"
        disabled={disabled}
        onClick={() => !disabled && (open ? setOpen(false) : openMenu())}
        onKeyDown={onKeyDown}
        aria-haspopup="listbox"
        aria-expanded={open}
        className={cn(
          "inline-flex w-full items-center justify-between gap-2 rounded-md border border-border bg-background/40 px-3 h-10 text-sm text-left text-foreground outline-none transition-colors hover:border-foreground/20 focus-visible:ring-1 focus-visible:ring-ring/50 disabled:opacity-50 disabled:pointer-events-none",
          mono && "font-mono",
          triggerClassName,
        )}
      >
        <span className={cn("truncate", selected ? selected.className : "text-muted-foreground")}>
          {selected ? selected.label : placeholder}
          {selected?.hint && <span className="text-muted-foreground ml-1">{selected.hint}</span>}
        </span>
        <ChevronDown
          className={cn(
            "size-4 shrink-0 text-muted-foreground transition-transform duration-150",
            open && "rotate-180",
          )}
        />
      </button>

      {open && (
        <ul
          role="listbox"
          className={cn(
            "absolute z-50 mt-1.5 w-full max-h-64 overflow-y-auto rounded-md border border-border bg-popover p-1 shadow-xl shadow-black/40 backdrop-blur-md animate-in fade-in-0 zoom-in-95 duration-100 scrollbar-thin scrollbar-thumb-muted scrollbar-track-transparent",
            mono && "font-mono",
          )}
        >
          {options.map((opt, i) => {
            const isSelected = opt.value === value;
            const isActive = i === active;
            return (
              <li key={opt.value} role="option" aria-selected={isSelected}>
                <button
                  type="button"
                  onClick={() => choose(opt.value)}
                  onMouseEnter={() => setActive(i)}
                  className={cn(
                    "flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm transition-colors",
                    isActive ? "bg-accent text-accent-foreground" : "text-foreground",
                    opt.className,
                  )}
                >
                  <Check
                    className={cn(
                      "size-3.5 shrink-0 text-primary",
                      isSelected ? "opacity-100" : "opacity-0",
                    )}
                  />
                  <span className="flex-1 truncate">{opt.label}</span>
                  {opt.hint && (
                    <span className="text-muted-foreground text-xs">{opt.hint}</span>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
