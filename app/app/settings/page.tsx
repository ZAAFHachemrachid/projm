"use client";

import { useRouter } from "next/navigation";
import { SettingsPanel } from "@/components/settings-panel";

// Thin route wrapper. The settings UI lives in SettingsPanel so the workspace
// can also render it as an in-page overlay (which keeps terminals mounted).
// Visiting /settings directly still works — closing returns home.
export default function SettingsPage() {
  const router = useRouter();
  return <SettingsPanel onClose={() => router.push("/")} />;
}
