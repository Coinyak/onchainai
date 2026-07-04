import Link from "next/link";

interface ToolkitEmptyStateProps {
  linked: boolean;
}

export function ToolkitEmptyState({ linked }: ToolkitEmptyStateProps) {
  return (
    <section
      className="toolkit-empty border border-border rounded-md p-lg mb-8"
      data-testid={linked ? "toolkit-empty-state-linked" : "toolkit-empty-state-not-linked"}
    >
      <h2 className="text-h2 mb-2">Your toolkit is empty</h2>
      <p className="text-secondary text-body-md mb-6 max-w-[640px]">
        Save MCP tools here to build export bundles and reopen them on any device.
      </p>

      <div className="toolkit-empty-paths">
        <div className="toolkit-empty-path" data-testid="toolkit-empty-path-website">
          <h3 className="text-body-md font-semibold mb-2">From the website</h3>
          <ol className="install-steps agent-link-steps">
            <li>Browse the directory.</li>
            <li>Open a tool from the list or preview panel.</li>
            <li>
              Click <strong>Save to Toolkit</strong> (★) on the tool card or preview bar.
            </li>
          </ol>
          <p className="text-body-sm text-secondary mt-2">
            Saved tools show up here immediately and in blueprint palettes.
          </p>
        </div>

        <div className="toolkit-empty-path" data-testid="toolkit-empty-path-agent">
          <h3 className="text-body-md font-semibold mb-2">From your coding tool</h3>
          {linked ? (
            <>
              <p
                className="text-body-sm toolkit-empty-agent-status mb-2"
                data-testid="toolkit-empty-agent-linked-status"
              >
                Agent linked — saves go straight to this toolkit.
              </p>
              <ol className="install-steps agent-link-steps">
                <li>While coding, ask your agent to save a tool you are evaluating.</li>
                <li>
                  Your agent saves it for you; the tool appears here with a <strong>From agent</strong>{" "}
                  badge.
                </li>
              </ol>
              <code className="toolkit-empty-example text-code" data-testid="toolkit-empty-agent-example">
                Save this MCP to my OnchainAI toolkit
              </code>
              <p className="mt-2">
                <Link
                  href="/connect#agent-sync"
                  className="text-primary text-body-sm underline-offset-2 hover:underline"
                  data-testid="toolkit-empty-manage-agent-link"
                >
                  Manage agent link →
                </Link>
              </p>
            </>
          ) : (
            <>
              <ol className="install-steps agent-link-steps">
                <li>Link Claude Code, Cursor, or another MCP client on Connect.</li>
                <li>In your agent, start linking and approve the one-time code on the website.</li>
                <li>
                  Ask your agent to save a tool — items appear here with a <strong>From agent</strong>{" "}
                  badge.
                </li>
              </ol>
              <code className="toolkit-empty-example text-code" data-testid="toolkit-empty-agent-example">
                Save the Coinbase MCP server to my toolkit
              </code>
            </>
          )}
        </div>
      </div>

      <div className="toolkit-empty-actions mt-6 flex flex-wrap gap-3">
        <Link href="/tools" className="agent-link-cta" data-testid="toolkit-empty-browse-cta">
          Browse tools
        </Link>
        {!linked && (
          <Link
            href="/connect#agent-sync"
            className="toolkit-agent-sync-cta"
            data-testid="toolkit-empty-link-agent-cta"
          >
            Link your agent
          </Link>
        )}
      </div>
    </section>
  );
}