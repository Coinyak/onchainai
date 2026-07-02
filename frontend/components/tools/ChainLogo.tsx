import { chainLogoPath } from "@/lib/chains";

interface ChainLogoProps {
  id: string;
  label: string;
  size?: number;
}

export function ChainLogo({ id, label, size = 36 }: ChainLogoProps) {
  return (
    <img
      className="chain-logo"
      src={chainLogoPath(id)}
      alt={label}
      title={label}
      width={size}
      height={size}
    />
  );
}