import { CopyButton } from "@/components/ui/CopyButton";

interface HighlightedCommandProps {
  command: string;
  showCopy?: boolean;
}

export function HighlightedCommand({ command, showCopy = true }: HighlightedCommandProps) {
  if (!command) return null;
  return (
    <div className="install-command-row">
      <code className="install-command">$ {command}</code>
      {showCopy && <CopyButton text={command} />}
    </div>
  );
}