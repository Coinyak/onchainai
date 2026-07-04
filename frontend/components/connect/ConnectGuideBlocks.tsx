import Link from "next/link";
import { HighlightedCommand } from "@/components/tools/HighlightedCommand";
import { CopyButton } from "@/components/ui/CopyButton";
import { CodingClientLogo } from "@/components/tools/CodingClientLogo";
import { logoIdForDeeplinkLabel } from "@/lib/coding-clients";
import type { ConnectGuideBlock } from "@/lib/install-guide";
import { copyLabelAria } from "@/lib/install-guide";

interface ConnectGuideBlocksProps {
  blocks: ConnectGuideBlock[];
  moreHref?: string;
}

export function ConnectGuideBlocks({ blocks, moreHref }: ConnectGuideBlocksProps) {
  return (
    <div className="connect-guide-blocks">
      {blocks.map((block, index) => (
        <ConnectGuideBlockView
          key={`${block.title ?? "block"}-${index}`}
          block={block}
          moreHref={moreHref}
        />
      ))}
    </div>
  );
}

function ConnectGuideBlockView({
  block,
  moreHref,
}: {
  block: ConnectGuideBlock;
  moreHref?: string;
}) {
  const copyAria = copyLabelAria(block.copyLabel);

  return (
    <div className="connect-guide-block">
      {block.title && <h4 className="connect-guide-block-title">{block.title}</h4>}
      <ul className="install-steps">
        {block.steps.map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ul>
      {block.deeplinkHref && block.deeplinkLabel && (
        <a
          href={block.deeplinkHref}
          className="connect-deeplink-btn"
          data-testid="connect-deeplink-btn"
        >
          {(() => {
            const logoId = logoIdForDeeplinkLabel(block.deeplinkLabel);
            return (
              <>
                {logoId && (
                  <CodingClientLogo
                    id={logoId}
                    label={block.deeplinkLabel}
                    size={18}
                    decorative
                  />
                )}
                <span>{block.deeplinkLabel}</span>
              </>
            );
          })()}
        </a>
      )}
      {block.copyText ? (
        <div className="tool-install-stack mt-3">
          <div className="tool-install">
            <HighlightedCommand
              command={block.copyText}
              showPrefix={block.showShellPrefix ?? false}
              showCopy={false}
            />
            <CopyButton text={block.copyText} label={copyAria} />
          </div>
        </div>
      ) : block.title === "More clients" && moreHref ? (
        <Link href={moreHref} className="connect-more-link mt-3">
          More clients →
        </Link>
      ) : null}
    </div>
  );
}