"use client";

import { useRouter } from "next/navigation";
import { ScanPanel } from "@/components/scan-panel";

// Thin route wrapper. The scan UI lives in ScanPanel so the workspace can also
// render it as an in-page overlay (which keeps terminals mounted). Visiting
// /scan directly still works — closing returns home.
export default function ScanPage() {
  const router = useRouter();
  return (
    <div className="p-6 lg:p-8 max-w-3xl mx-auto">
      <ScanPanel onClose={() => router.push("/")} />
    </div>
  );
}
