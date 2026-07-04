import { ConnectOnchainAiMcpCard } from "@/components/tools/ConnectOnchainAiMcpCard";
import { QuickInstallPromoCard } from "@/components/tools/QuickInstallPromoCard";

interface PromoCardsProps {
  mcpEndpoint: string;
}

export function PromoCards({ mcpEndpoint }: PromoCardsProps) {
  return (
    <section className="promo-cards-section" aria-label="Connect OnchainAI">
      <div className="promo-cards-grid min-w-0">
        <ConnectOnchainAiMcpCard mcpEndpoint={mcpEndpoint} />
        <QuickInstallPromoCard />
      </div>
    </section>
  );
}