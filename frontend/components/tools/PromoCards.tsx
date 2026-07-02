import Link from "next/link";
import { CopyButton } from "@/components/ui/CopyButton";

interface PromoCardsProps {
  mcpEndpoint: string;
}

export function PromoCards({ mcpEndpoint }: PromoCardsProps) {
  return (
    <div className="promo-cards">
      <Link href="/submit" className="promo-card promo-card-submit no-underline">
        <h3 className="text-h3 text-primary">Submit a Tool</h3>
        <p className="text-body-sm text-secondary">
          Suggest a crypto MCP, CLI, SDK, or API for operator review.
        </p>
      </Link>
      <div className="promo-card promo-card-mcp">
        <h3 className="text-h3 text-primary">Connect via MCP</h3>
        <div className="install-command-row">
          <code className="install-command text-code">{mcpEndpoint}</code>
          <CopyButton text={mcpEndpoint} />
        </div>
      </div>
    </div>
  );
}