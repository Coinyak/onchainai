interface BadgeProps {
  children: React.ReactNode;
  variant?: "verified" | "official" | "neutral" | "x402" | "community" | "risk";
  className?: string;
}

export function Badge({ children, variant = "neutral", className = "" }: BadgeProps) {
  const variantClass = {
    verified: "badge badge-verified",
    official: "badge badge-official",
    neutral: "badge badge-neutral",
    x402: "badge badge-x402",
    community: "badge badge-community",
    risk: "badge badge-neutral",
  }[variant];

  return <span className={`${variantClass} ${className}`.trim()}>{children}</span>;
}