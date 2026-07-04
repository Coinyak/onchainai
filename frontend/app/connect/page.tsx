import type { Metadata } from "next";
import { Suspense } from "react";
import { ConnectPageContent } from "@/components/connect/ConnectPageContent";
import { SITE_ORIGIN } from "@/lib/site";

export const metadata: Metadata = {
  title: "Connect OnchainAI to Your Agent | OnchainAI",
  description:
    "Universal MCP install plus setup for Claude, Cursor, VS Code, ChatGPT, Codex, Windsurf, Gemini, and other agents.",
  alternates: {
    canonical: `${SITE_ORIGIN}/connect`,
  },
  openGraph: {
    title: "Connect OnchainAI MCP",
    description:
      "One hub for every MCP client: universal install, deeplinks, and per-client setup steps.",
    url: `${SITE_ORIGIN}/connect`,
    type: "website",
  },
};

export default function ConnectPage() {
  return (
    <Suspense fallback={null}>
      <ConnectPageContent />
    </Suspense>
  );
}