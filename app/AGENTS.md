<!-- BEGIN:nextjs-agent-rules -->
# This is NOT the Next.js you know

This version has breaking changes — APIs, conventions, and file structure may all differ from your training data. Read the relevant guide in `node_modules/next/dist/docs/` before writing any code. Heed deprecation notices.
<!-- END:nextjs-agent-rules -->

# APP

Next.js 16 Tauri webview frontend for `projm` — shadcn/ui, React 19, Tailwind CSS 4, xterm.js terminal.

## STRUCTURE

```
app/
├── app/
│   ├── layout.tsx          # Root: Geist font, dark class, full-height body
│   ├── page.tsx            # Main dashboard (44KB) — largest surface
│   ├── settings/page.tsx   # Settings panel (1258 lines)
│   ├── scan/page.tsx       # Directory scan trigger
│   ├── projects/page.tsx   # Project list
│   ├── diagnostics/page.tsx# Env diagnostics display
│   └── not-found.tsx       # 404
├── components/
│   ├── app-sidebar.tsx     # Nav sidebar
│   └── ui/                 # 12 shadcn/ui components
├── hooks/
│   └── use-mobile.ts       # Mobile detection
├── lib/
│   └── utils.ts            # cn() utility
└── globals.css             # Tailwind 4 entry
```

## WHERE TO LOOK

| Component | Path | Role |
|-----------|------|------|
| page.tsx | `app/app/page.tsx` | Main dashboard, Tauri command invocations |
| settings | `app/app/settings/page.tsx` | Configuration UI |
| sidebar | `app/components/ui/sidebar.tsx` | Reusable shadcn sidebar (21KB) |
| terminal | `app/components/ui/terminal.tsx` | xterm.js terminal wrapper |
| select | `app/components/ui/select.tsx` | shadcn select with search |

## CONVENTIONS

- `dark` class on `<html>` — always dark mode
- Geist Sans + Geist Mono via `next/font`
- shadcn/ui components in `components/ui/` (CVA + tailwind-merge pattern)
- Tauri IPC via `@tauri-apps/api` invoke
- Terminal via `@xterm/xterm` with `@xterm/addon-fit`
- Tailwind CSS 4 with PostCSS
- React Server Components by default (app router)

## ANTI-PATTERNS

- Do NOT use Pages Router — app router only
- Do NOT add raw CSS outside `globals.css` — use Tailwind utilities
- Do NOT hardcode Tauri command names — import from `@tauri-apps/api`
- Do NOT add client components unless interactivity requires it
- Next.js 16 — verify API compatibility before using new features
