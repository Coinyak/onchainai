import { ExternalLink } from "lucide-react";
import type { ToolOfficialLink } from "@/lib/api";

interface OfficialLinksListProps {
  links: ToolOfficialLink[];
}

export function OfficialLinksList({ links }: OfficialLinksListProps) {
  if (!links.length) return null;
  return (
    <section className="detail-section">
      <h2 className="text-h2 mb-3">Official links</h2>
      <ul className="official-links-list">
        {links.map((link) => (
          <li key={link.id}>
            <a
              href={link.url}
              target="_blank"
              rel="noopener noreferrer"
              className="official-link-item no-underline"
            >
              {link.link_type} · {link.verification_status}
              <ExternalLink size={14} aria-hidden />
            </a>
          </li>
        ))}
      </ul>
    </section>
  );
}