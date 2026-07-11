import type { BrowserDataPayload } from "@/lib/api";

/**
 * Server-rendered default catalog for crawlers and no-JS.
 * Client ToolsBrowser hydrates the interactive list; this ensures tool names
 * and links exist in the initial HTML (review P3).
 */
export function ServerHomeToolList({ data }: { data: BrowserDataPayload }) {
  if (!data.tools.length) return null;

  return (
    <section
      className="ssr-home-tool-list px-gutter md:px-6 pb-4"
      aria-label="Tools"
      data-testid="ssr-home-tool-list"
    >
      <p className="text-body-sm text-secondary mb-3">
        {data.total} tools in the directory
      </p>
      <ul className="tool-list">
        {data.tools.map((tool) => (
          <li key={tool.slug} className="tool-card">
            <a
              href={`/tools/${encodeURIComponent(tool.slug)}`}
              className="tool-card-link no-underline text-inherit"
            >
              <div className="tool-card-inner">
                <div className="tool-card-body">
                  <h3 className="tool-name">{tool.name}</h3>
                  {tool.description ? (
                    <p className="tool-desc">{tool.description}</p>
                  ) : null}
                  <p className="tool-meta text-body-sm text-secondary">
                    {tool.type.toUpperCase()}
                    {tool.function ? ` · ${tool.function}` : ""}
                  </p>
                </div>
              </div>
            </a>
          </li>
        ))}
      </ul>
    </section>
  );
}
