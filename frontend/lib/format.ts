export function monogramFromName(name: string): string {
  const trimmed = name.trim();
  if (!trimmed) return "?";
  const parts = trimmed.split(/\s+/).filter(Boolean);
  if (parts.length >= 2) {
    return (parts[0][0] + parts[1][0]).toUpperCase();
  }
  return trimmed.slice(0, 2).toUpperCase();
}

export function timeAgo(iso: string | null | undefined): string {
  if (!iso) return "—";
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return "—";
  const diff = Date.now() - then;
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;
  const weeks = Math.floor(days / 7);
  if (weeks < 5) return `${weeks}w ago`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months}mo ago`;
  return `${Math.floor(days / 365)}y ago`;
}

export function statusBadgeLabel(status: string): string {
  switch (status) {
    case "verified": return "Verified";
    case "official": return "Official";
    default: return "Community";
  }
}

export function typeBadgeLabel(toolType: string): string {
  if (toolType === "x402") return "x402";
  return toolType.toUpperCase();
}

export function displayInstallCommand(tool: {
  safe_copy_command?: string | null;
  install_command?: string | null;
}): string {
  return tool.safe_copy_command?.trim() || tool.install_command?.trim() || "";
}

export function formatGithubStars(count: number): string {
  return `GitHub · ${count.toLocaleString("en-US")} stars`;
}