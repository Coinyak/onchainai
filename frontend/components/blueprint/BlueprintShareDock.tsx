"use client";

import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { Share2, X } from "lucide-react";
import type { BlueprintEdge, BlueprintNode, PublicTool } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import {
  buildDraftAgentMarkdown,
  captureBlueprintContent,
  captureBlueprintViewport,
  BLUEPRINT_EXPORT_PLATFORMS,
  DEFAULT_EXPORT_PLATFORM,
  type BlueprintExportPlatform,
} from "@/lib/blueprint-export";
import { CodingClientLogo } from "@/components/tools/CodingClientLogo";
import {
  shouldAutoRegenSharePrompt,
  shouldShowStaleShareBanner,
} from "@/lib/blueprint-share-regen-core.mjs";
import {
  blueprintCanvasFingerprint,
  loadSharePromptDraft,
  saveSharePromptDraft,
} from "@/lib/blueprint-share-storage";

const CANVAS_AUTO_REGEN_MS = 500;

type ShareTab = "prompt" | "image";
type CaptureTarget = "viewport" | "full" | null;

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
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState<ShareTab>("prompt");
  const [platform, setPlatform] = useState<BlueprintExportPlatform>(DEFAULT_EXPORT_PLATFORM);
  const [promptText, setPromptText] = useState("");
  const [baselineMarkdown, setBaselineMarkdown] = useState("");
  const [baselineFingerprint, setBaselineFingerprint] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [capturing, setCapturing] = useState<CaptureTarget>(null);
  const [copyState, setCopyState] = useState<"idle" | "copied" | "error">("idle");
  const [copyLiveMessage, setCopyLiveMessage] = useState("");

  const canvasFingerprint = useMemo(() => {
    if (!open) return "";
    return blueprintCanvasFingerprint(title, nodes, edges);
  }, [open, title, nodes, edges]);

  const visible = isDraft || isAuthenticated;
  const hasNodes = nodes.length > 0;
  const isDirty = promptText !== baselineMarkdown;
  const isCanvasStale =
    open && hasNodes && baselineFingerprint !== "" && canvasFingerprint !== baselineFingerprint;
  const showStaleBanner = shouldShowStaleShareBanner(isCanvasStale, isDirty);

  const applyFreshPrompt = useCallback(
    (platformOverride?: BlueprintExportPlatform) => {
    if (!hasNodes) {
      setPromptText("");
      setBaselineMarkdown("");
      setBaselineFingerprint("");
      setError(null);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const effectivePlatform = platformOverride ?? platform;
      const markdown = buildDraftAgentMarkdown(title, nodes, edges, toolsBySlug, effectivePlatform);
      setPromptText(markdown);
      setBaselineMarkdown(markdown);
      setBaselineFingerprint(canvasFingerprint);
      saveSharePromptDraft(blueprintId, canvasFingerprint, markdown);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to generate prompt");
    } finally {
      setLoading(false);
    }
  }, [blueprintId, canvasFingerprint, edges, hasNodes, nodes, platform, title, toolsBySlug]);

  const ensurePromptOnOpen = useCallback(() => {
    if (!hasNodes) {
      setError(null);
      return;
    }

    if (baselineFingerprint && canvasFingerprint === baselineFingerprint && promptText) {
      return;
    }

    const stored = loadSharePromptDraft(blueprintId);
    if (stored && stored.fingerprint === canvasFingerprint) {
      setPromptText(stored.markdown);
      setBaselineMarkdown(stored.markdown);
      setBaselineFingerprint(stored.fingerprint);
      setError(null);
      return;
    }

    applyFreshPrompt();
  }, [
    applyFreshPrompt,
    baselineFingerprint,
    blueprintId,
    canvasFingerprint,
    hasNodes,
    promptText,
  ]);

  const handleToggleOpen = useCallback(() => {
    setOpen((prev) => !prev);
  }, []);

  const wasOpenRef = useRef(false);
  const autoRegenTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (open && !wasOpenRef.current) {
      ensurePromptOnOpen();
    }
    wasOpenRef.current = open;
  }, [open, ensurePromptOnOpen]);

  useEffect(() => {
    if (
      !shouldAutoRegenSharePrompt({
        open,
        hasNodes,
        loading,
        baselineFingerprint,
        canvasFingerprint,
        isDirty,
      })
    ) {
      return;
    }

    if (autoRegenTimerRef.current) {
      clearTimeout(autoRegenTimerRef.current);
    }

    autoRegenTimerRef.current = setTimeout(() => {
      applyFreshPrompt();
    }, CANVAS_AUTO_REGEN_MS);

    return () => {
      if (autoRegenTimerRef.current) {
        clearTimeout(autoRegenTimerRef.current);
        autoRegenTimerRef.current = null;
      }
    };
  }, [
    open,
    hasNodes,
    loading,
    baselineFingerprint,
    canvasFingerprint,
    isDirty,
    applyFreshPrompt,
  ]);

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

  const handleRegenerate = useCallback(() => {
    if (isDirty || isCanvasStale) {
      const message =
        isDirty && isCanvasStale
          ? "Replace your edits and refresh the prompt from the current blueprint?"
          : isDirty
            ? "Replace your edits with a fresh draft from the current blueprint?"
            : "Regenerate the prompt from the current blueprint?";
      if (!window.confirm(message)) {
        return;
      }
    }
    applyFreshPrompt();
  }, [applyFreshPrompt, isCanvasStale, isDirty]);

  const handleCopyPrompt = useCallback(async () => {
    if (!promptText.trim()) return;
    try {
      await navigator.clipboard.writeText(promptText);
      const draftFingerprint = baselineFingerprint || canvasFingerprint;
      if (draftFingerprint) {
        saveSharePromptDraft(blueprintId, draftFingerprint, promptText);
      }
      setCopyState("copied");
      setCopyLiveMessage("Prompt copied. Attach PNG from Image tab.");
      window.setTimeout(() => {
        setCopyState("idle");
        setCopyLiveMessage("");
      }, 3000);
    } catch {
      setCopyState("error");
      window.setTimeout(() => setCopyState("idle"), 2000);
    }
  }, [baselineFingerprint, blueprintId, canvasFingerprint, promptText]);

  const handleDownloadMd = useCallback(() => {
    if (!promptText.trim()) return;
    const blob = new Blob([promptText], { type: "text/markdown;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.download = "blueprint-agent.md";
    link.href = url;
    link.click();
    URL.revokeObjectURL(url);
  }, [promptText]);

  const capturePng = useCallback(async (target: "viewport" | "full") => {
    const viewportEl = document.querySelector<HTMLElement>(".blueprint-canvas-viewport");
    if (!viewportEl) {
      setError("Canvas viewport not found");
      return;
    }

    setCapturing(target);
    setError(null);
    try {
      const dataUrl =
        target === "viewport"
          ? await captureBlueprintViewport(viewportEl)
          : await captureBlueprintContent(viewportEl);
      setPreviewUrl(dataUrl);
      setTab("image");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to capture blueprint");
    } finally {
      setCapturing(null);
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
      <div className="sr-only" aria-live="polite">
        {copyLiveMessage}
      </div>

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
                <p className="blueprint-share-empty">
                  Add nodes to the canvas to generate a prompt.
                </p>
              ) : (
                <>
                  {showStaleBanner ? (
                    <p className="blueprint-share-stale" role="status">
                      Blueprint changed — Regenerate to refresh the prompt (your edits will be
                      replaced).
                    </p>
                  ) : null}
                  <p className="blueprint-share-hint">
                    Auto-generated draft — edit before sending.
                  </p>
                  <div className="blueprint-share-platform-row" role="radiogroup" aria-label="Target coding tool">
                    <span className="blueprint-share-platform-label">Tool:</span>
                    <div className="blueprint-share-platform-chips">
                      {BLUEPRINT_EXPORT_PLATFORMS.map((p) => (
                        <button
                          key={p.id}
                          type="button"
                          role="radio"
                          aria-checked={platform === p.id}
                          className={`blueprint-share-platform-chip${platform === p.id ? " blueprint-share-platform-chip-active" : ""}`}
                          data-testid={`blueprint-share-platform-${p.id}`}
                          onClick={() => {
                            setPlatform(p.id);
                            if (hasNodes && !isDirty) {
                              setTimeout(() => applyFreshPrompt(p.id), 0);
                            }
                          }}
                        >
                          <CodingClientLogo
                            id={p.logoId}
                            label={p.label}
                            size={16}
                            decorative
                          />
                          {p.label}
                        </button>
                      ))}
                    </div>
                  </div>
                  <textarea
                    className="blueprint-share-prompt-edit"
                    data-testid="blueprint-share-prompt-edit"
                    rows={16}
                    value={promptText}
                    onChange={(event) => {
                      const next = event.target.value;
                      setPromptText(next);
                      if (baselineFingerprint) {
                        saveSharePromptDraft(blueprintId, baselineFingerprint, next);
                      }
                    }}
                    readOnly={loading}
                    spellCheck={false}
                  />
                  <p className="blueprint-share-hint">
                    Also download the PNG from the Image tab and attach it with this prompt.
                  </p>
                  <div className="blueprint-share-actions">
                    <button
                      type="button"
                      className="blueprint-share-btn blueprint-share-btn-secondary"
                      data-testid="blueprint-share-regenerate"
                      onClick={handleRegenerate}
                      disabled={loading || !hasNodes}
                    >
                      Regenerate
                    </button>
                    <button
                      type="button"
                      className="blueprint-share-btn blueprint-share-btn-secondary"
                      data-testid="blueprint-download-md"
                      onClick={handleDownloadMd}
                      disabled={loading || !promptText.trim()}
                    >
                      Download .md
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
              <p className="blueprint-share-hint">
                Viewport capture shows what&apos;s on screen. ## Flow and ## Order in the prompt
                describe the full blueprint.
              </p>
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
              <div className="blueprint-share-actions">
                <button
                  type="button"
                  className="blueprint-share-btn blueprint-share-btn-secondary"
                  data-testid="blueprint-download-png"
                  onClick={() => void capturePng("viewport")}
                  disabled={capturing !== null}
                >
                  {capturing === "viewport" ? "Capturing…" : "Download viewport PNG"}
                </button>
                <button
                  type="button"
                  className="blueprint-share-btn blueprint-share-btn-secondary"
                  data-testid="blueprint-download-full-png"
                  onClick={() => void capturePng("full")}
                  disabled={capturing !== null}
                >
                  {capturing === "full" ? "Capturing…" : "Download full canvas PNG"}
                </button>
              </div>
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