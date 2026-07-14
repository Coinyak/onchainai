import Link from "next/link";
import { getPublicDashboardServer } from "@/lib/server-api";

// 404 is served for every unmatched path (bots/scanners hammer these). Drop the
// old `force-dynamic` so this is statically prerendered + ISR: a bogus URL is a
// CDN hit, not a per-request serverless render + Railway fetch. The 1h segment
// floor is capped down to ~5m by the popular-tools fetch (revalidate: 300),
// which is fine — regeneration is shared across all 404s, not per request.
export const revalidate = 3600;

export default async function NotFound() {
  let popularTools: { slug: string; name: string }[] = [];

  const dashboard = await getPublicDashboardServer(6).catch(() => null);
  if (dashboard) {
    popularTools = dashboard.popular_tools.map((tool) => ({
      slug: tool.slug,
      name: tool.name,
    }));
  }

  return (
    <div className="px-gutter md:px-8 py-10 max-w-[720px] mx-auto" data-testid="not-found-page">
      <h1 className="text-h1 mb-3">Page not found</h1>
      <p className="text-secondary text-body-md leading-relaxed mb-8">
        This page does not exist or may have moved. Search the directory or browse popular tools below.
      </p>

      <form action="/tools" method="get" className="mb-8">
        <label htmlFor="not-found-search" className="sr-only">
          Search tools
        </label>
        <input
          id="not-found-search"
          type="search"
          name="q"
          placeholder="Search tools..."
          className="search-input w-full h-12 px-4 text-body-md rounded-md border border-border bg-neutral-bg text-primary outline-none focus:border-tertiary"
          data-testid="not-found-search"
        />
      </form>

      {popularTools.length > 0 && (
        <section className="mb-8" aria-labelledby="not-found-popular-heading">
          <h2 id="not-found-popular-heading" className="text-h2 mb-3">
            Popular tools
          </h2>
          <div className="empty-state-suggestions">
            {popularTools.map((tool) => (
              <Link
                key={tool.slug}
                href={`/tools/${tool.slug}`}
                className="empty-state-suggestion"
                data-testid={`not-found-popular-${tool.slug}`}
              >
                {tool.name}
              </Link>
            ))}
          </div>
        </section>
      )}

      <div className="empty-state-actions">
        <Link href="/" className="empty-state-clear-btn">
          Back to home
        </Link>
        <Link href="/tools" className="empty-state-submit-btn">
          Browse all tools
        </Link>
      </div>
    </div>
  );
}