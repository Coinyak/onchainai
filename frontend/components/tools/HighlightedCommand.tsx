import { CopyButton } from "@/components/ui/CopyButton";
import { tokenizeCommand, tokenClass } from "@/lib/tokenizeCommand";

interface HighlightedCommandProps {
  command: string;
  showCopy?: boolean;
  showPrefix?: boolean;
  className?: string;
}

export function HighlightedCommand({
  command,
  showCopy = true,
  showPrefix = true,
  className = "install-cmd",
}: HighlightedCommandProps) {
  if (!command) return null;

  const tokens = tokenizeCommand(command);

  return (
    <>
      <code className={className}>
        {showPrefix && <span className="install-prefix">$ </span>}
        {tokens.map((token, idx) => (
          <span key={`${idx}-${token.text}`}>
            {idx > 0 && " "}
            <span className={tokenClass(token.kind)}>{token.text}</span>
          </span>
        ))}
      </code>
      {showCopy && <CopyButton text={command} />}
    </>
  );
}