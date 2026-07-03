"use client";

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useSensor,
  useSensors,
  useDroppable,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Compass } from "lucide-react";
import {
  createBlueprint,
  getBlueprint,
  getToolBySlug,
  updateBlueprint,
  type Blueprint,
  type BlueprintNode,
  type Tool,
} from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { LoginModal } from "@/components/auth/LoginModal";
import {
  BLUEPRINT_DRAFT_ID,
  clearLocalBlueprintDraft,
  createEmptyDraft,
  getLocalBlueprintDraftSnapshot,
  saveLocalBlueprintDraft,
  type LocalBlueprintDraft,
} from "@/lib/blueprint-storage";
import {
  BLUEPRINT_MAX_NODES,
  clampCoord,
  newNodeId,
  pointerToCanvasCoords,
} from "@/lib/blueprint-utils";
import { timeAgo, typeBadgeLabel } from "@/lib/format";
import { BlueprintPalette } from "@/components/blueprint/BlueprintPalette";
import { BlueprintNodeView } from "@/components/blueprint/BlueprintNodeView";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { Badge } from "@/components/ui/Badge";

interface BlueprintEditorProps {
  blueprintId: string;
}

type SaveState = "idle" | "pending" | "saving" | "saved" | "error";

interface BlueprintEditorWorkspaceProps {
  blueprintId: string;
  isDraft: boolean;
  initialTitle: string;
  initialNodes: BlueprintNode[];
  initialSavedAt: string;
  readOnlyLayout: boolean;
}

function useReadOnlyLayout(): boolean {
  const [readOnly, setReadOnly] = useState(false);

  useEffect(() => {
    const mq = window.matchMedia("(max-width: 1023px)");
    const update = () => setReadOnly(mq.matches);
    update();
    mq.addEventListener("change", update);
    return () => mq.removeEventListener("change", update);
  }, []);

  return readOnly;
}

function CanvasDropZone({
  children,
  viewportRef,
  onPointerDown,
  onPointerMove,
  onPointerUp,
}: {
  children: React.ReactNode;
  viewportRef: React.RefObject<HTMLDivElement | null>;
  onPointerDown?: (e: React.PointerEvent<HTMLDivElement>) => void;
  onPointerMove?: (e: React.PointerEvent<HTMLDivElement>) => void;
  onPointerUp?: (e: React.PointerEvent<HTMLDivElement>) => void;
}) {
  const { setNodeRef, isOver } = useDroppable({
    id: "blueprint-canvas",
  });

  return (
    <div
      ref={(el) => {
        setNodeRef(el);
        if (viewportRef) viewportRef.current = el;
      }}
      className={`blueprint-canvas-viewport${isOver ? " blueprint-canvas-viewport-over" : ""}`}
      data-testid="blueprint-canvas"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
    >
      {children}
    </div>
  );
}

function BlueprintEditorWorkspace({
  blueprintId,
  isDraft,
  initialTitle,
  initialNodes,
  initialSavedAt,
  readOnlyLayout,
}: BlueprintEditorWorkspaceProps) {
  const router = useRouter();
  const queryClient = useQueryClient();
  const { isAuthenticated } = useAuth();

  const [title, setTitle] = useState(initialTitle);
  const [nodes, setNodes] = useState(initialNodes);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [saveState, setSaveState] = useState<SaveState>("saved");
  const [savedAt, setSavedAt] = useState<string | null>(initialSavedAt);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [liveMessage, setLiveMessage] = useState("");
  const [activeDragTool, setActiveDragTool] = useState<Tool | null>(null);
  const [promoting, setPromoting] = useState(false);

  const viewportRef = useRef<HTMLDivElement | null>(null);
  const panRef = useRef<{
    active: boolean;
    startX: number;
    startY: number;
    scrollLeft: number;
    scrollTop: number;
  } | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const promoteAttemptedRef = useRef(false);

  const readOnly =
    readOnlyLayout || promoting || (isDraft ? false : !isAuthenticated);

  const titleRef = useRef(title);
  const nodesRef = useRef(nodes);

  useEffect(() => {
    titleRef.current = title;
    nodesRef.current = nodes;
  }, [title, nodes]);

  const toolSlugs = useMemo(
    () => [...new Set(nodes.filter((n) => n.kind === "tool" && n.slug).map((n) => n.slug!))],
    [nodes],
  );

  const toolsQuery = useQuery({
    queryKey: ["blueprint-tools", toolSlugs],
    queryFn: async () => {
      const entries = await Promise.all(
        toolSlugs.map(async (slug) => {
          try {
            const tool = await getToolBySlug(slug);
            return [slug, tool] as const;
          } catch {
            return [slug, null] as const;
          }
        }),
      );
      return Object.fromEntries(entries) as Record<string, Tool | null>;
    },
    enabled: toolSlugs.length > 0,
  });

  const toolsBySlug = toolsQuery.data ?? {};

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
  );

  const persistDraft = useCallback((nextTitle: string, nextNodes: BlueprintNode[]) => {
    const draft: LocalBlueprintDraft = {
      title: nextTitle,
      nodes: nextNodes,
      updatedAt: new Date().toISOString(),
    };
    saveLocalBlueprintDraft(draft);
    setSavedAt(draft.updatedAt);
    setSaveState("saved");
  }, []);

  const saveMutation = useMutation({
    mutationFn: async (payload: { title: string; nodes: BlueprintNode[] }) => {
      if (isDraft) {
        persistDraft(payload.title, payload.nodes);
        return null;
      }
      return updateBlueprint(blueprintId, payload);
    },
    onMutate: () => {
      setSaveState("saving");
      setSaveError(null);
    },
    onSuccess: (data) => {
      if (data) {
        setSavedAt(data.updated_at);
        queryClient.setQueryData(["blueprint", blueprintId], data);
      }
      setSaveState("saved");
    },
    onError: (err: Error) => {
      setSaveState("error");
      setSaveError(err.message);
    },
  });

  const scheduleSave = useCallback(
    (nextTitle: string, nextNodes: BlueprintNode[]) => {
      if (readOnly) return;
      setSaveState("pending");
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      saveTimerRef.current = setTimeout(() => {
        saveMutation.mutate({ title: nextTitle, nodes: nextNodes });
      }, 2000);
    },
    [readOnly, saveMutation],
  );

  useEffect(
    () => () => {
      if (!saveTimerRef.current) return;
      clearTimeout(saveTimerRef.current);
      saveTimerRef.current = null;
      if (readOnlyLayout) return;
      if (isDraft) {
        persistDraft(titleRef.current, nodesRef.current);
        return;
      }
      saveMutation.mutate({ title: titleRef.current, nodes: nodesRef.current });
    },
    [isDraft, persistDraft, readOnlyLayout, saveMutation],
  );

  const promoteDraft = useCallback(async () => {
    if (!isDraft || !isAuthenticated || promoteAttemptedRef.current) return;
    promoteAttemptedRef.current = true;
    setPromoting(true);
    if (saveTimerRef.current) {
      clearTimeout(saveTimerRef.current);
      saveTimerRef.current = null;
    }
    const snapshotTitle = titleRef.current;
    const snapshotNodes = nodesRef.current;
    try {
      const created = await createBlueprint({ title: snapshotTitle, nodes: snapshotNodes });
      clearLocalBlueprintDraft();
      queryClient.invalidateQueries({ queryKey: ["blueprints"] });
      router.replace(`/blueprints/${created.id}`);
    } catch {
      setPromoting(false);
      // Keep draft mode; do not retry promote in a loop.
    }
  }, [isDraft, isAuthenticated, queryClient, router]);

  useEffect(() => {
    if (!isDraft || !isAuthenticated) return;
    const id = window.setTimeout(() => {
      void promoteDraft();
    }, 0);
    return () => window.clearTimeout(id);
  }, [isDraft, isAuthenticated, promoteDraft]);

  const updateNodes = useCallback(
    (updater: (prev: BlueprintNode[]) => BlueprintNode[]) => {
      setNodes((prev) => {
        const next = updater(prev);
        scheduleSave(title, next);
        return next;
      });
    },
    [scheduleSave, title],
  );

  const updateTitle = useCallback(
    (nextTitle: string) => {
      setTitle(nextTitle);
      scheduleSave(nextTitle, nodes);
    },
    [nodes, scheduleSave],
  );

  const addToolNode = useCallback(
    (tool: Tool, x: number, y: number) => {
      if (readOnly) return;
      if (nodes.length >= BLUEPRINT_MAX_NODES) {
        setSaveError(`Blueprints accept at most ${BLUEPRINT_MAX_NODES} nodes.`);
        setSaveState("error");
        return;
      }
      const node: BlueprintNode = {
        id: newNodeId(),
        kind: "tool",
        slug: tool.slug,
        x: clampCoord(x),
        y: clampCoord(y),
      };
      updateNodes((prev) => [...prev, node]);
      setSelectedId(node.id);
      setLiveMessage(`Added ${tool.name} to canvas.`);
    },
    [nodes.length, readOnly, updateNodes],
  );

  const addToolAtViewportCenter = useCallback(
    (tool: Tool) => {
      const viewport = viewportRef.current;
      if (!viewport) return;
      const x = viewport.scrollLeft + viewport.clientWidth / 2 - 110;
      const y = viewport.scrollTop + viewport.clientHeight / 2 - 32;
      addToolNode(tool, x, y);
    },
    [addToolNode],
  );

  const addNoteNode = useCallback(() => {
    if (readOnly) return;
    if (nodes.length >= BLUEPRINT_MAX_NODES) {
      setSaveError(`Blueprints accept at most ${BLUEPRINT_MAX_NODES} nodes.`);
      setSaveState("error");
      return;
    }
    const viewport = viewportRef.current;
    const x = viewport
      ? viewport.scrollLeft + viewport.clientWidth / 2 - 110
      : 200;
    const y = viewport
      ? viewport.scrollTop + viewport.clientHeight / 2 - 40
      : 200;
    const node: BlueprintNode = {
      id: newNodeId(),
      kind: "note",
      text: "",
      x: clampCoord(x),
      y: clampCoord(y),
    };
    updateNodes((prev) => [...prev, node]);
    setSelectedId(node.id);
    setLiveMessage("Added note to canvas.");
  }, [nodes.length, readOnly, updateNodes]);

  const removeNode = useCallback(
    (id: string) => {
      if (readOnly) return;
      updateNodes((prev) => prev.filter((n) => n.id !== id));
      if (selectedId === id) setSelectedId(null);
      setLiveMessage("Node removed.");
    },
    [readOnly, selectedId, updateNodes],
  );

  const updateNodeText = useCallback(
    (id: string, text: string) => {
      if (readOnly) return;
      updateNodes((prev) =>
        prev.map((n) => (n.id === id ? { ...n, text } : n)),
      );
    },
    [readOnly, updateNodes],
  );

  const moveNode = useCallback(
    (id: string, dx: number, dy: number) => {
      if (readOnly) return;
      updateNodes((prev) =>
        prev.map((n) => {
          if (n.id !== id) return n;
          return {
            ...n,
            x: clampCoord(n.x + dx),
            y: clampCoord(n.y + dy),
          };
        }),
      );
      setLiveMessage("Node moved.");
    },
    [readOnly, updateNodes],
  );

  const handleDragStart = (event: DragStartEvent) => {
    const data = event.active.data.current;
    if (data?.type === "palette-tool") {
      setActiveDragTool(data.tool as Tool);
    }
  };

  const handleDragEnd = (event: DragEndEvent) => {
    setActiveDragTool(null);
    const { active, over, delta } = event;
    const data = active.data.current;

    if (data?.type === "palette-tool" && over?.id === "blueprint-canvas") {
      const tool = data.tool as Tool;
      const translated = active.rect.current.translated;
      const viewport = viewportRef.current;
      if (translated && viewport) {
        const cx = translated.left + translated.width / 2;
        const cy = translated.top + translated.height / 2;
        const coords = pointerToCanvasCoords(cx, cy, viewport);
        addToolNode(tool, coords.x - 110, coords.y - 32);
      }
      return;
    }

    if (data?.type === "canvas-node") {
      const nodeId = data.nodeId as string;
      const node = nodes.find((n) => n.id === nodeId);
      if (!node) return;
      const newX = clampCoord(node.x + delta.x);
      const newY = clampCoord(node.y + delta.y);
      updateNodes((prev) =>
        prev.map((n) =>
          n.id === nodeId ? { ...n, x: newX, y: newY } : n,
        ),
      );
      setLiveMessage("Node moved.");
    }
  };

  const handleCanvasPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest("[data-testid='blueprint-node']")) return;
    const viewport = e.currentTarget;
    panRef.current = {
      active: true,
      startX: e.clientX,
      startY: e.clientY,
      scrollLeft: viewport.scrollLeft,
      scrollTop: viewport.scrollTop,
    };
    viewport.setPointerCapture(e.pointerId);
    setSelectedId(null);
  };

  const handleCanvasPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    const pan = panRef.current;
    const viewport = viewportRef.current;
    if (!pan?.active || !viewport) return;
    viewport.scrollLeft = pan.scrollLeft - (e.clientX - pan.startX);
    viewport.scrollTop = pan.scrollTop - (e.clientY - pan.startY);
  };

  const handleCanvasPointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    const viewport = e.currentTarget;
    if (panRef.current?.active && viewport.hasPointerCapture(e.pointerId)) {
      viewport.releasePointerCapture(e.pointerId);
    }
    panRef.current = null;
  };

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!selectedId || readOnly) return;
      const step = e.shiftKey ? 40 : 8;
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        moveNode(selectedId, -step, 0);
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        moveNode(selectedId, step, 0);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        moveNode(selectedId, 0, -step);
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        moveNode(selectedId, 0, step);
      } else if (e.key === "Delete" || e.key === "Backspace") {
        const target = e.target as HTMLElement;
        if (target.tagName === "TEXTAREA" || target.tagName === "INPUT") return;
        e.preventDefault();
        removeNode(selectedId);
      } else if (e.key === "Enter") {
        const node = nodes.find((n) => n.id === selectedId);
        if (node?.kind === "note") {
          const textarea = document.querySelector<HTMLTextAreaElement>(
            `[data-node-id="${selectedId}"] textarea`,
          );
          textarea?.focus();
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [selectedId, readOnly, moveNode, removeNode, nodes]);

  const saveLabel = useMemo(() => {
    if (saveState === "saving" || saveState === "pending") return "Saving...";
    if (saveState === "error") return saveError ?? "Save failed";
    if (savedAt) return `Saved · ${timeAgo(savedAt)}`;
    return "Saved";
  }, [saveState, saveError, savedAt]);

  return (
    <div className="blueprint-editor">
      {readOnlyLayout && (
        <div className="blueprint-mobile-banner" role="status">
          Editing works best on desktop. This view is read-only on smaller screens.
        </div>
      )}

      <header className="blueprint-toolbar">
        <Link href="/blueprints" className="blueprint-toolbar-back">
          Back to list
        </Link>
        <input
          type="text"
          className="blueprint-toolbar-title"
          value={title}
          onChange={(e) => updateTitle(e.target.value)}
          readOnly={readOnly}
          aria-label="Blueprint title"
        />
        <div className="blueprint-toolbar-actions">
          <button
            type="button"
            className="blueprint-toolbar-btn"
            data-testid="blueprint-add-note"
            onClick={addNoteNode}
            disabled={readOnly}
          >
            Add note
          </button>
          {selectedId && !readOnly && (
            <button
              type="button"
              className="blueprint-toolbar-btn blueprint-toolbar-btn-danger"
              onClick={() => removeNode(selectedId)}
            >
              Delete
            </button>
          )}
          <span
            className="blueprint-save-state"
            data-testid="blueprint-save-state"
            aria-live="polite"
          >
            {saveLabel}
          </span>
        </div>
      </header>

      <div className="blueprint-editor-body">
        <DndContext
          sensors={sensors}
          onDragStart={handleDragStart}
          onDragEnd={handleDragEnd}
        >
          {!readOnlyLayout && (
            <BlueprintPalette readOnly={readOnly} onAddTool={addToolAtViewportCenter} />
          )}

          <CanvasDropZone
            viewportRef={viewportRef}
            onPointerDown={handleCanvasPointerDown}
            onPointerMove={handleCanvasPointerMove}
            onPointerUp={handleCanvasPointerUp}
          >
            <div className="blueprint-canvas-surface">
              {nodes.length === 0 && (
                <div className="blueprint-empty-state">
                  <Compass size={32} strokeWidth={1.5} color="#4B4B4B" />
                  <p>Drag tools from the left to start planning</p>
                </div>
              )}
              {nodes.map((node) => (
                <BlueprintNodeView
                  key={node.id}
                  node={node}
                  tool={node.slug ? toolsBySlug[node.slug] : null}
                  toolMissing={!!node.slug && toolsBySlug[node.slug] === null}
                  selected={selectedId === node.id}
                  readOnly={readOnly}
                  onSelect={setSelectedId}
                  onRemove={removeNode}
                  onTextChange={updateNodeText}
                />
              ))}
            </div>
          </CanvasDropZone>

          <DragOverlay dropAnimation={null}>
            {activeDragTool ? (
              <div className="blueprint-node blueprint-node-tool blueprint-node-overlay">
                <ToolLogo
                  name={activeDragTool.name}
                  logoUrl={activeDragTool.logo_url}
                  logoMonogram={activeDragTool.logo_monogram}
                  size={32}
                />
                <span className="blueprint-node-tool-text">
                  <span className="blueprint-node-tool-name">{activeDragTool.name}</span>
                  <Badge variant="neutral">{typeBadgeLabel(activeDragTool.type)}</Badge>
                </span>
              </div>
            ) : null}
          </DragOverlay>
        </DndContext>
      </div>

      <div className="sr-only" aria-live="polite">
        {liveMessage}
      </div>
    </div>
  );
}

function BlueprintDraftEditor({ readOnlyLayout }: { readOnlyLayout: boolean }) {
  const [initialDraft] = useState(
    () => getLocalBlueprintDraftSnapshot() ?? createEmptyDraft(),
  );

  return (
    <BlueprintEditorWorkspace
      blueprintId={BLUEPRINT_DRAFT_ID}
      isDraft
      initialTitle={initialDraft.title}
      initialNodes={initialDraft.nodes}
      initialSavedAt={initialDraft.updatedAt}
      readOnlyLayout={readOnlyLayout}
    />
  );
}

function BlueprintGuestGate() {
  const router = useRouter();
  const [dismissed, setDismissed] = useState(false);

  return (
    <>
      <LoginModal
        open={!dismissed}
        onClose={() => {
          setDismissed(true);
          router.push("/blueprints");
        }}
      />
      <div className="blueprint-editor-guest">
        <p>Sign in to edit this blueprint, or try a local draft.</p>
        <Link href={`/blueprints/${BLUEPRINT_DRAFT_ID}`} className="toolkit-secondary-link">
          Open local draft
        </Link>
      </div>
    </>
  );
}

export function BlueprintEditor({ blueprintId }: BlueprintEditorProps) {
  const { isAuthenticated } = useAuth();
  const readOnlyLayout = useReadOnlyLayout();
  const isDraft = blueprintId === BLUEPRINT_DRAFT_ID;

  const blueprintQuery = useQuery({
    queryKey: ["blueprint", blueprintId],
    queryFn: () => getBlueprint(blueprintId),
    enabled: !isDraft && isAuthenticated,
    retry: false,
  });

  if (isDraft) {
    return <BlueprintDraftEditor readOnlyLayout={readOnlyLayout} />;
  }

  if (!isAuthenticated) {
    return <BlueprintGuestGate />;
  }

  if (blueprintQuery.isLoading) {
    return <div className="blueprint-editor-loading">Loading blueprint...</div>;
  }

  if (blueprintQuery.isError || !blueprintQuery.data) {
    return <BlueprintGuestGate />;
  }

  const blueprint: Blueprint = blueprintQuery.data;

  return (
    <BlueprintEditorWorkspace
      key={blueprint.id}
      blueprintId={blueprintId}
      isDraft={false}
      initialTitle={blueprint.title}
      initialNodes={blueprint.nodes}
      initialSavedAt={blueprint.updated_at}
      readOnlyLayout={readOnlyLayout}
    />
  );
}