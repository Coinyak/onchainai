"use client";

import { useCallback, useEffect, useId, useRef, useState } from "react";
import { Share2, X } from "lucide-react";
import {
  getBlueprintAgentExport,
  type BlueprintEdge,
  type BlueprintNode,
  type PublicTool,
} from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { buildDraftAgentMarkdown, captureBlueprintViewport } from "@/lib/blueprint-export";

type ShareTab = "prompt" | "image";

export interface BlueprintShareDockProps {
  blueprintId: string;
  isDraft: boolean;
  title: string;
  nodes: BlueprintNode[];
  edges: BlueprintEdge[];
  toolsBySlug: Record<string, PublicTool | null>;
  readOnlyLayout: boolean;
}

export function BlueprintShareDock({
  blueprintId,
  isDraft,
  title,
  nodes,
  edges,
  toolsBySlug,
  readOnlyLayout,
}: BlueprintShareDockProps) {
  const { isAuthenticated } = useAuth();
  const panelId = useId();
  const loadGenerationRef = useRef(0);
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState<ShareTab>("prompt");
  const [promptText, setPromptText] = useState("");
  const [baselineMarkdown, setBaselineMarkdown] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [capturing, setCapturing] = useState(false);
  const [copyState, setCopyState] = useState<"idle" | "copied" | "error">("idle");

  const visible = isDraft || isAuthenticated;
  const hasNodes = nodes.length > 0;
  const isDirty = promptText !== baselineMarkdown;

  const generatePrompt = useCallback(async () => {
    if (!hasNodes) {
      setPromptText("");
      setBaselineMarkdown("");
      setError(null);
      return;
    }

    const generation = ++loadGenerationRef.current;
    setLoading(true);
    setError(null);

    try {
      let markdown: string;
      if (isDraft) {
        markdown = buildDraftAgentMarkdown(title, nodes, edges, toolsBySlug);
      } else {
        const response = await getBlueprintAgentExport(blueprintId);
        markdown = response.markdown;
      }

      if (generation !== loadGenerationRef.current) return;
      setPromptText(markdown);
      setBaselineMarkdown(markdown);
    } catch (err) {
      if (generation !== loadGenerationRef.current) return;
      setError(err instanceof Error ? err.message : "Failed to generate prompt");
    } finally {
      if (generation === loadGenerationRef.current) {
        setLoading(false);
      }
    }
  }, [blueprintId, edges, hasNodes, isDraft, nodes, title, toolsBySlug]);

  const resetPromptState = useCallback(() => {
    setPromptText("");
    setBaselineMarkdown("");
    setError(null);
    setPreviewUrl(null);
    setCopyState("idle");
  }, []);

  const handleToggleOpen = useCallback(() => {
    setOpen((prev) => {
      const next = !prev;
      if (next) {
        resetPromptState();
        void generatePrompt();
      }
      return next;
    });
  }, [generatePrompt, resetPromptState]);

  useEffect(() => {
    if (!open) return;

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open]);

  const handleRegenerate = useCallback(async () => {
    if (
      isDirty &&
      !window.confirm("Replace your edits with a fresh draft from the current blueprint?")
    ) {
      return;
    }
    await generatePrompt();
  }, [generatePrompt, isDirty]);

  const handleCopyPrompt = useCallback(async () => {
    if (!promptText.trim()) return;
    try {
      await navigator.clipboard.writeText(promptText);
      setCopyState("copied");
      window.setTimeout(() => setCopyState("idle"), 2000);
    } catch {
      setCopyState("error");
      window.setTimeout(() => setCopyState("idle"), 2000);
    }
  }, [promptText]);

  const handleDownloadPng = useCallback(async () => {
    const viewportEl = document.querySelector<HTMLElement>(".blueprint-canvas-viewport");
    if (!viewportEl) {
      setError("Canvas viewport not found");
      return;
    }

    setCapturing(true);
    setError(null);
    try {
      const dataUrl = await captureBlueprintViewport(viewportEl);
      setPreviewUrl(dataUrl);
      setTab("image");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to capture blueprint");
    } finally {
      setCapturing(false);
    }
  }, []);

  if (!visible) {
    return null;
  }

  return (
    <div
      className="blueprint-share-dock"
      data-testid="blueprint-share-dock"
      onPointerDown={(event) => event.stopPropagation()}
    >
      {open ? (
        <div
          id={panelId}
          className="blueprint-share-panel"
          data-testid="blueprint-share-panel"
          role="dialog"
          aria-label="Share blueprint"
        >
          <div className="blueprint-share-panel-header">
            <div className="blueprint-share-tabs" role="tablist" aria-label="Share format">
              <button
                type="button"
                role="tab"
                aria-selected={tab === "prompt"}
                className={`blueprint-share-tab${tab === "prompt" ? " blueprint-share-tab-active" : ""}`}
                onClick={() => setTab("prompt")}
              >
                Prompt
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={tab === "image"}
                className={`blueprint-share-tab${tab === "image" ? " blueprint-share-tab-active" : ""}`}
                onClick={() => setTab("image")}
              >
                Image
              </button>
            </div>
            <button
              type="button"
              className="blueprint-share-panel-close"
              aria-label="Close share panel"
              onClick={() => setOpen(false)}
            >
              <X size={18} aria-hidden="true" />
            </button>
          </div>

          {tab === "prompt" ? (
            <div className="blueprint-share-prompt">
              {!hasNodes ? (
                <p className="blueprint-share-empty">Add tools to generate a prompt.</p>
              ) : (
                <>
                  <p className="blueprint-share-hint">
                    Auto-generated draft — edit before sending.
                  </p>
                  <textarea
                    className="blueprint-share-prompt-edit"
                    data-testid="blueprint-share-prompt-edit"
                    rows={16}
                    value={promptText}
                    onChange={(event) => setPromptText(event.target.value)}
                    readOnly={loading}
                    spellCheck={false}
                  />
                  <div className="blueprint-share-actions">
                    <button
                      type="button"
                      className="blueprint-share-btn blueprint-share-btn-secondary"
                      data-testid="blueprint-share-regenerate"
                      onClick={() => void handleRegenerate()}
                      disabled={loading || !hasNodes}
                    >
                      Regenerate
                    </button>
                    <button
                      type="button"
                      className="blueprint-share-btn blueprint-share-copy-btn"
                      data-testid="blueprint-copy-prompt"
                      onClick={() => void handleCopyPrompt()}
                      disabled={loading || !promptText.trim()}
                    >
                      {copyState === "copied"
                        ? "Copied"
                        : copyState === "error"
                          ? "Copy failed"
                          : "Copy prompt"}
                    </button>
                  </div>
                </>
              )}
            </div>
          ) : (
            <div className="blueprint-share-image">
              {previewUrl ? (
                // eslint-disable-next-line @next/next/no-img-element -- data URL from html-to-image capture; next/image cannot preview dynamic blob/data URLs
                <img
                  src={previewUrl}
                  alt="Blueprint preview"
                  className="blueprint-share-image-preview"
                />
              ) : (
                <p className="blueprint-share-empty">
                  Capture the current canvas to preview a clean PNG export.
                </p>
              )}
              <button
                type="button"
                className="blueprint-share-btn blueprint-share-btn-secondary"
                data-testid="blueprint-download-png"
                onClick={() => void handleDownloadPng()}
                disabled={capturing}
              >
                {capturing ? "Capturing…" : "Download PNG"}
              </button>
            </div>
          )}

          {error ? <p className="blueprint-share-error">{error}</p> : null}
          {readOnlyLayout ? (
            <p className="blueprint-share-readonly-note">
              Read-only layout — prompt copy and PNG export are still available.
            </p>
          ) : null}
        </div>
      ) : null}

      <button
        type="button"
        className="blueprint-share-dock-fab"
        aria-expanded={open}
        aria-controls={open ? panelId : undefined}
        onClick={handleToggleOpen}
      >
        <Share2 size={18} aria-hidden="true" />
        <span>Share</span>
      </button>
    </div>
  );
}