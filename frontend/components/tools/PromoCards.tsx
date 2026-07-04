import { ConnectOnchainAiMcpCard } from "@/components/tools/ConnectOnchainAiMcpCard";
import { PluginSkillPromoCard } from "@/components/tools/PluginSkillPromoCard";

interface PromoCardsProps {
  mcpEndpoint: string;
}

export function PromoCards({ mcpEndpoint }: PromoCardsProps) {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4 min-w-0">
      <PluginSkillPromoCard />
      <ConnectOnchainAiMcpCard mcpEndpoint={mcpEndpoint} />
    </div>
  );
}