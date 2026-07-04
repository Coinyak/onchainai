import type { Metadata } from "next";
import { X402HubContent } from "@/components/tools/X402HubContent";

export const metadata: Metadata = {
  title: "x402 Tools Directory — OnchainAI",
  description:
    "Discover live x402 payment endpoints for AI agents, or list your own. Every listing is probed for a real 402 handshake — machine-checked, no gatekeeping.",
  alternates: { canonical: "/x402" },
};

export default function X402HubPage() {
  return <X402HubContent />;
}
