import Link from "next/link";
import { ConnectOnchainAiMcpCard } from "@/components/tools/ConnectOnchainAiMcpCard";

interface PromoCardsProps {
  mcpEndpoint: string;
}

export function PromoCards({ mcpEndpoint }: PromoCardsProps) {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4 min-w-0">
      <div className="promo-card border border-[#E5E5E5] rounded-lg p-6 bg-white">
        <h3 className="text-[16px] font-semibold mb-2">Suggest a Tool</h3>
        <p className="text-[14px] text-[#6B6B6B] mb-4 leading-relaxed">
          Know a crypto MCP, CLI, SDK, API, or x402 tool we should review? Send
          it for operator review.
        </p>
        <Link
          href="/submit"
          className="inline-flex items-center justify-center h-10 px-4 rounded-lg border border-border-strong bg-neutral-bg text-primary text-[14px] font-medium no-underline hover:bg-neutral-surface"
        >
          Suggest →
        </Link>
      </div>
      <ConnectOnchainAiMcpCard mcpEndpoint={mcpEndpoint} />
    </div>
  );
}