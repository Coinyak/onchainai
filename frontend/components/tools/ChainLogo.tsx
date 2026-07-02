import { chainLogoPath } from "@/lib/chains";

interface ChainLogoProps {
  id: string;
  label: string;
  size?: number;
  className?: string;
}

export function ChainLogo({ id, label, size = 36, className = "chain-logo" }: ChainLogoProps) {
  return (
    <img
      className={className}
      src={chainLogoPath(id)}
      alt={label}
      title={label}
      width={size}
      height={size}
    />
  );
}