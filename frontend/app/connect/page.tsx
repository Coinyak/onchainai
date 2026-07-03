import type { Metadata } from "next";
import { ConnectPageContent } from "@/components/connect/ConnectPageContent";
import { SITE_ORIGIN } from "@/lib/site";

export const metadata: Metadata = {
  title: "Connect OnchainAI MCP — Client Setup | OnchainAI",
  description:
    "Install the OnchainAI MCP search server in Claude, Cursor, VS Code, ChatGPT, Codex, Windsurf, Gemini, and other MCP clients.",
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
  return <ConnectPageContent />;
}