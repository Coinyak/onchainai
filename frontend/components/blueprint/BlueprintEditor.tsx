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
  type BlueprintEdge,
  type BlueprintNode,
  type PublicTool,
  type PublicToolSummary,
} from "@/lib/api";
import { CHAIN_CATALOG } from "@/lib/chains";
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
  BLUEPRINT_EDGE_COLORS,
  BLUEPRINT_MAX_EDGES,
  BLUEPRINT_MAX_NODES,
  BLUEPRINT_NODE_CHAIN_SIZE,
  clampCoord,
  getNodeAnchor,
  initialToolNodeChains,
  newEdgeId,
  newNodeId,
  normalizeToolNodeChains,
  pointerToCanvasCoords,
  pruneEdgesForNodes,
  type BlueprintEdgeStyle,
} from "@/lib/blueprint-utils";
import { timeAgo, typeBadgeLabel } from "@/lib/format";
import { BlueprintEdgeInspector } from "@/components/blueprint/BlueprintEdgeInspector";
import { BlueprintEdgesLayer } from "@/components/blueprint/BlueprintEdgesLayer";
import { BlueprintPalette } from "@/components/blueprint/BlueprintPalette";
import { BlueprintNodeView } from "@/components/blueprint/BlueprintNodeView";
import { BlueprintShareDock } from "@/components/blueprint/BlueprintShareDock";
import { ToolLogo } from "@/components/tools/ToolLogo";
import { Badge } from "@/components/ui/Badge";

interface BlueprintEditorProps {
  blueprintId: string;
}

type SaveState = "idle" | "pending" | "saving" | "saved" | "error";

const BLUEPRINT_EDGE_PREFS_KEY = "onchainai-blueprint-edge-prefs";

interface BlueprintEdgePrefs {
  style: BlueprintEdgeStyle;
  color: string;
}

function loadBlueprintEdgePrefs(): BlueprintEdgePrefs {
  if (typeof window === "undefined") {
    return { style: "arrow", color: BLUEPRINT_EDGE_COLORS[0].value };
  }
  try {
    const raw = window.localStorage.getItem(BLUEPRINT_EDGE_PREFS_KEY);
    if (!raw) {
      return { style: "arrow", color: BLUEPRINT_EDGE_COLORS[0].value };
    }
    const parsed = JSON.parse(raw) as Partial<BlueprintEdgePrefs>;
    const style = parsed.style === "solid" ? "solid" : "arrow";
    const color =
      typeof parsed.color === "string" &&
      BLUEPRINT_EDGE_COLORS.some((option) => option.value === parsed.color)
        ? parsed.color
        : BLUEPRINT_EDGE_COLORS[0].value;
    return { style, color };
  } catch {
    return { style: "arrow", color: BLUEPRINT_EDGE_COLORS[0].value };
  }
}

function saveBlueprintEdgePrefs(prefs: BlueprintEdgePrefs): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(BLUEPRINT_EDGE_PREFS_KEY, JSON.stringify(prefs));
}

interface BlueprintEditorWorkspaceProps {
  blueprintId: string;
  isDraft: boolean;
  initialTitle: string;
  initialNodes: BlueprintNode[];
  initialEdges: BlueprintEdge[];
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
  initialEdges,
  initialSavedAt,
  readOnlyLayout,
}: BlueprintEditorWorkspaceProps) {
  const router = useRouter();
  const queryClient = useQueryClient();
  const { isAuthenticated } = useAuth();

  const [title, setTitle] = useState(initialTitle);
  const [nodes, setNodes] = useState(initialNodes);
  const [edges, setEdges] = useState(initialEdges);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedEdgeId, setSelectedEdgeId] = useState<string | null>(null);
  const [linkingFromId, setLinkingFromId] = useState<string | null>(null);
  const [linkingFromSide, setLinkingFromSide] = useState<"in" | "out" | null>(null);
  const [linkPointer, setLinkPointer] = useState<{ x: number; y: number } | null>(
    null,
  );
  const [chainsPopoverOpenId, setChainsPopoverOpenId] = useState<string | null>(
    null,
  );
  const [edgeStyle, setEdgeStyle] = useState<BlueprintEdgeStyle>(
    () => loadBlueprintEdgePrefs().style,
  );
  const [edgeColor, setEdgeColor] = useState<string>(
    () => loadBlueprintEdgePrefs().color,
  );
  const [saveState, setSaveState] = useState<SaveState>("saved");
  const [savedAt, setSavedAt] = useState<string | null>(initialSavedAt);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [liveMessage, setLiveMessage] = useState("");
  const [activeDragTool, setActiveDragTool] = useState<PublicTool | PublicToolSummary | null>(null);
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
  const edgesRef = useRef(edges);

  useEffect(() => {
    titleRef.current = title;
    nodesRef.current = nodes;
    edgesRef.current = edges;
  }, [title, nodes, edges]);

  const chainLabelById = useMemo(
    () => Object.fromEntries(CHAIN_CATALOG.map((chain) => [chain.id, chain.label])),
    [],
  );

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
      return Object.fromEntries(entries) as Record<string, PublicTool | null>;
    },
    enabled: toolSlugs.length > 0,
  });

  const toolsBySlug = useMemo(
    () => toolsQuery.data ?? {},
    [toolsQuery.data],
  );

  const selectedEdge = useMemo(
    () => edges.find((edge) => edge.id === selectedEdgeId) ?? null,
    [edges, selectedEdgeId],
  );

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
  );

  const persistDraft = useCallback(
    (nextTitle: string, nextNodes: BlueprintNode[], nextEdges: BlueprintEdge[]) => {
      const draft: LocalBlueprintDraft = {
        title: nextTitle,
        nodes: nextNodes,
        edges: nextEdges,
        updatedAt: new Date().toISOString(),
      };
      saveLocalBlueprintDraft(draft);
      setSavedAt(draft.updatedAt);
      setSaveState("saved");
    },
    [],
  );

  const saveMutation = useMutation({
    mutationFn: async (payload: {
      title: string;
      nodes: BlueprintNode[];
      edges: BlueprintEdge[];
    }) => {
      if (isDraft) {
        persistDraft(payload.title, payload.nodes, payload.edges);
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
    (nextTitle: string, nextNodes: BlueprintNode[], nextEdges: BlueprintEdge[]) => {
      if (readOnly) return;
      setSaveState("pending");
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      saveTimerRef.current = setTimeout(() => {
        saveMutation.mutate({ title: nextTitle, nodes: nextNodes, edges: nextEdges });
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
        persistDraft(titleRef.current, nodesRef.current, edgesRef.current);
        return;
      }
      saveMutation.mutate({
        title: titleRef.current,
        nodes: nodesRef.current,
        edges: edgesRef.current,
      });
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
      const created = await createBlueprint({
        title: snapshotTitle,
        nodes: snapshotNodes,
        edges: edgesRef.current,
      });
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
        const nextEdges = pruneEdgesForNodes(edgesRef.current, next);
        if (nextEdges.length !== edgesRef.current.length) {
          setEdges(nextEdges);
          setSelectedEdgeId(null);
        }
        scheduleSave(title, next, nextEdges);
        return next;
      });
    },
    [scheduleSave, title],
  );

  const updateEdges = useCallback(
    (updater: (prev: BlueprintEdge[]) => BlueprintEdge[]) => {
      setEdges((prev) => {
        const next = updater(prev);
        scheduleSave(title, nodes, next);
        return next;
      });
    },
    [nodes, scheduleSave, title],
  );

  const updateTitle = useCallback(
    (nextTitle: string) => {
      setTitle(nextTitle);
      scheduleSave(nextTitle, nodes, edges);
    },
    [edges, nodes, scheduleSave],
  );

  const addToolNode = useCallback(
    (tool: PublicTool | PublicToolSummary, x: number, y: number) => {
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
        chains: initialToolNodeChains(tool.chains),
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
    (tool: PublicTool | PublicToolSummary) => {
      const viewport = viewportRef.current;
      if (!viewport) return;
      const x = viewport.scrollLeft + viewport.clientWidth / 2 - 110;
      const y = viewport.scrollTop + viewport.clientHeight / 2 - 32;
      addToolNode(tool, x, y);
    },
    [addToolNode],
  );

  const addChainNode = useCallback(
    (chainId: string, x: number, y: number) => {
      if (readOnly) return;
      if (nodes.length >= BLUEPRINT_MAX_NODES) {
        setSaveError(`Blueprints accept at most ${BLUEPRINT_MAX_NODES} nodes.`);
        setSaveState("error");
        return;
      }
      const node: BlueprintNode = {
        id: newNodeId(),
        kind: "chain",
        chainId,
        x: clampCoord(x),
        y: clampCoord(y),
      };
      updateNodes((prev) => [...prev, node]);
      setSelectedId(node.id);
      setSelectedEdgeId(null);
      setLiveMessage(`Added ${chainLabelById[chainId] ?? chainId} sticker.`);
    },
    [chainLabelById, nodes.length, readOnly, updateNodes],
  );

  const addChainAtViewportCenter = useCallback(
    (chain: { id: string }) => {
      const viewport = viewportRef.current;
      if (!viewport) return;
      const x = viewport.scrollLeft + viewport.clientWidth / 2 - BLUEPRINT_NODE_CHAIN_SIZE / 2;
      const y = viewport.scrollTop + viewport.clientHeight / 2 - BLUEPRINT_NODE_CHAIN_SIZE / 2;
      addChainNode(chain.id, x, y);
    },
    [addChainNode],
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
      updateEdges((prev) => prev.filter((edge) => edge.fromId !== id && edge.toId !== id));
      if (selectedId === id) setSelectedId(null);
      if (linkingFromId === id) {
        setLinkingFromId(null);
        setLinkPointer(null);
      }
      if (chainsPopoverOpenId === id) setChainsPopoverOpenId(null);
      setSelectedEdgeId(null);
      setLiveMessage("Node removed.");
    },
    [chainsPopoverOpenId, linkingFromId, readOnly, selectedId, updateEdges, updateNodes],
  );

  const removeEdge = useCallback(
    (id: string) => {
      if (readOnly) return;
      updateEdges((prev) => prev.filter((edge) => edge.id !== id));
      if (selectedEdgeId === id) setSelectedEdgeId(null);
      setLiveMessage("Link removed.");
    },
    [readOnly, selectedEdgeId, updateEdges],
  );

  const addEdgeBetween = useCallback(
    (fromId: string, toId: string) => {
      if (readOnly || fromId === toId) return;
      if (edges.length >= BLUEPRINT_MAX_EDGES) {
        setSaveError(`Blueprints accept at most ${BLUEPRINT_MAX_EDGES} links.`);
        setSaveState("error");
        return;
      }
      const duplicate = edges.some(
        (edge) =>
          (edge.fromId === fromId && edge.toId === toId) ||
          (edge.fromId === toId && edge.toId === fromId),
      );
      if (duplicate) {
        setLiveMessage("These nodes are already linked.");
        return;
      }
      const edge: BlueprintEdge = {
        id: newEdgeId(),
        fromId,
        toId,
        style: edgeStyle,
        color: edgeColor,
      };
      updateEdges((prev) => [...prev, edge]);
      setSelectedEdgeId(edge.id);
      setSelectedId(null);
      setLiveMessage("Link added.");
    },
    [edgeColor, edgeStyle, edges, readOnly, updateEdges],
  );

  const cancelLinking = useCallback(() => {
    setLinkingFromId(null);
    setLinkingFromSide(null);
    setLinkPointer(null);
  }, []);

  const handleNodeSelect = useCallback((id: string) => {
    setSelectedId(id);
    setSelectedEdgeId(null);
    setChainsPopoverOpenId(null);
  }, []);

  const handleEdgeStyleChange = useCallback(
    (style: BlueprintEdgeStyle) => {
      setEdgeStyle(style);
      saveBlueprintEdgePrefs({ style, color: edgeColor });
      if (!selectedEdgeId || readOnly) return;
      updateEdges((prev) =>
        prev.map((edge) => (edge.id === selectedEdgeId ? { ...edge, style } : edge)),
      );
    },
    [edgeColor, readOnly, selectedEdgeId, updateEdges],
  );

  const handleEdgeColorChange = useCallback(
    (color: string) => {
      setEdgeColor(color);
      saveBlueprintEdgePrefs({ style: edgeStyle, color });
      if (!selectedEdgeId || readOnly) return;
      updateEdges((prev) =>
        prev.map((edge) => (edge.id === selectedEdgeId ? { ...edge, color } : edge)),
      );
    },
    [edgeStyle, readOnly, selectedEdgeId, updateEdges],
  );

  const completeLinking = useCallback(
    (clientX: number, clientY: number) => {
      if (!linkingFromId || !linkingFromSide || readOnly) {
        cancelLinking();
        return;
      }
      const target = document.elementFromPoint(clientX, clientY);
      const portEl = target?.closest("[data-port]");
      const nodeEl = target?.closest("[data-testid='blueprint-node']");

      const connectOutToIn = (fromId: string, toId: string) => {
        if (fromId !== toId) addEdgeBetween(fromId, toId);
      };

      if (portEl) {
        const nodeId = portEl.getAttribute("data-node-id");
        const port = portEl.getAttribute("data-port");
        if (!nodeId || nodeId === linkingFromId) {
          cancelLinking();
          return;
        }
        if (linkingFromSide === "out" && port === "in") {
          connectOutToIn(linkingFromId, nodeId);
        } else if (linkingFromSide === "in" && port === "out") {
          connectOutToIn(nodeId, linkingFromId);
        }
      } else if (nodeEl) {
        const nodeId = nodeEl.getAttribute("data-node-id");
        if (nodeId && nodeId !== linkingFromId) {
          if (linkingFromSide === "out") {
            connectOutToIn(linkingFromId, nodeId);
          } else {
            connectOutToIn(nodeId, linkingFromId);
          }
        }
      }
      cancelLinking();
    },
    [addEdgeBetween, cancelLinking, linkingFromId, linkingFromSide, readOnly],
  );

  const handlePortPointerDown = useCallback(
    (
      nodeId: string,
      side: "out" | "in",
      e: React.PointerEvent<HTMLButtonElement>,
    ) => {
      if (readOnly) return;
      e.stopPropagation();
      e.preventDefault();
      const node = nodes.find((n) => n.id === nodeId);
      if (!node) return;
      const anchor = getNodeAnchor(node, side);
      setLinkingFromId(nodeId);
      setLinkingFromSide(side);
      setLinkPointer(anchor);
      setSelectedId(null);
      setSelectedEdgeId(null);
      setChainsPopoverOpenId(null);
      setLiveMessage("Drag to a target node to link.");
    },
    [nodes, readOnly],
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

  const updateNodeChains = useCallback(
    (id: string, chains: string[]) => {
      if (readOnly) return;
      updateNodes((prev) =>
        prev.map((n) => {
          if (n.id !== id || n.kind !== "tool") return n;
          const tool = n.slug ? toolsBySlug[n.slug] : null;
          const normalized = tool
            ? normalizeToolNodeChains(chains, tool.chains)
            : chains;
          return { ...n, chains: normalized };
        }),
      );
      setLiveMessage("Chain selection updated.");
    },
    [readOnly, toolsBySlug, updateNodes],
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
      setActiveDragTool(data.tool as PublicTool | PublicToolSummary);
    }
  };

  const handleDragEnd = (event: DragEndEvent) => {
    setActiveDragTool(null);
    const { active, over, delta } = event;
    const data = active.data.current;

    if (data?.type === "palette-tool" && over?.id === "blueprint-canvas") {
      const tool = data.tool as PublicTool | PublicToolSummary;
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

    if (data?.type === "palette-chain" && over?.id === "blueprint-canvas") {
      const chain = data.chain as { id: string };
      const translated = active.rect.current.translated;
      const viewport = viewportRef.current;
      if (translated && viewport) {
        const cx = translated.left + translated.width / 2;
        const cy = translated.top + translated.height / 2;
        const coords = pointerToCanvasCoords(cx, cy, viewport);
        addChainNode(
          chain.id,
          coords.x - BLUEPRINT_NODE_CHAIN_SIZE / 2,
          coords.y - BLUEPRINT_NODE_CHAIN_SIZE / 2,
        );
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
    if ((e.target as HTMLElement).closest("[data-port]")) return;
    if ((e.target as HTMLElement).closest("[data-testid='blueprint-node']")) return;
    if ((e.target as HTMLElement).closest(".blueprint-edge-hit, .blueprint-edge-handle")) return;
    if (linkingFromId) {
      cancelLinking();
      setLiveMessage("Link cancelled.");
      return;
    }
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
    setSelectedEdgeId(null);
    setChainsPopoverOpenId(null);
  };

  const handleCanvasPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (linkingFromId) {
      const viewport = viewportRef.current;
      if (!viewport) return;
      setLinkPointer(pointerToCanvasCoords(e.clientX, e.clientY, viewport));
      return;
    }
    const pan = panRef.current;
    const viewport = viewportRef.current;
    if (!pan?.active || !viewport) return;
    viewport.scrollLeft = pan.scrollLeft - (e.clientX - pan.startX);
    viewport.scrollTop = pan.scrollTop - (e.clientY - pan.startY);
  };

  const handleCanvasPointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    if (linkingFromId) return;
    const viewport = e.currentTarget;
    if (panRef.current?.active && viewport.hasPointerCapture(e.pointerId)) {
      viewport.releasePointerCapture(e.pointerId);
    }
    panRef.current = null;
  };

  useEffect(() => {
    if (!linkingFromId) return;

    const onPointerMove = (e: PointerEvent) => {
      const viewport = viewportRef.current;
      if (!viewport) return;
      setLinkPointer(pointerToCanvasCoords(e.clientX, e.clientY, viewport));
    };

    const onPointerUp = (e: PointerEvent) => {
      completeLinking(e.clientX, e.clientY);
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    return () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
    };
  }, [completeLinking, linkingFromId]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (readOnly) return;
      if (e.key === "Escape") {
        if (linkingFromId) {
          e.preventDefault();
          cancelLinking();
          setLiveMessage("Link cancelled.");
        }
        return;
      }
      if (selectedEdgeId && (e.key === "Delete" || e.key === "Backspace")) {
        const target = e.target as HTMLElement;
        if (target.tagName === "TEXTAREA" || target.tagName === "INPUT") return;
        e.preventDefault();
        removeEdge(selectedEdgeId);
        return;
      }
      if (!selectedId) return;
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
  }, [
    cancelLinking,
    linkingFromId,
    moveNode,
    nodes,
    readOnly,
    removeEdge,
    removeNode,
    selectedEdgeId,
    selectedId,
  ]);

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

      <header className="blueprint-toolbar" style={{ minHeight: 52 }}>
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
        <BlueprintEdgeInspector
          visible={!!selectedEdgeId}
          edgeStyle={selectedEdge?.style ?? edgeStyle}
          edgeColor={selectedEdge?.color ?? edgeColor}
          selectedEdgeId={selectedEdgeId}
          readOnly={readOnly}
          onStyleChange={handleEdgeStyleChange}
          onColorChange={handleEdgeColorChange}
          onDeleteEdge={() => selectedEdgeId && removeEdge(selectedEdgeId)}
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
            <BlueprintPalette
              readOnly={readOnly}
              onAddTool={addToolAtViewportCenter}
              onAddChain={addChainAtViewportCenter}
            />
          )}

          <CanvasDropZone
            viewportRef={viewportRef}
            onPointerDown={handleCanvasPointerDown}
            onPointerMove={handleCanvasPointerMove}
            onPointerUp={handleCanvasPointerUp}
          >
            <div className="blueprint-canvas-surface">
              <BlueprintEdgesLayer
                edges={edges}
                nodes={nodes}
                selectedEdgeId={selectedEdgeId}
                readOnly={readOnly}
                onSelectEdge={(id) => {
                  setSelectedEdgeId(id);
                  setSelectedId(null);
                  cancelLinking();
                  setChainsPopoverOpenId(null);
                }}
              />
              {linkingFromId && linkPointer && (() => {
                const fromNode = nodes.find((n) => n.id === linkingFromId);
                if (!fromNode) return null;
                const start = getNodeAnchor(fromNode, "out");
                return (
                  <svg
                    className="blueprint-rubber-band-layer"
                    aria-hidden="true"
                    data-testid="blueprint-rubber-band"
                  >
                    <line
                      x1={start.x}
                      y1={start.y}
                      x2={linkPointer.x}
                      y2={linkPointer.y}
                      stroke={edgeColor}
                      strokeWidth={2}
                      strokeDasharray="6 4"
                    />
                  </svg>
                );
              })()}
              {nodes.length === 0 && (
                <div className="blueprint-empty-state">
                  <Compass size={32} strokeWidth={1.5} color="#4B4B4B" />
                  <p>Drag tools or network stickers from the left to start planning</p>
                </div>
              )}
              {nodes.map((node) => (
                <BlueprintNodeView
                  key={node.id}
                  node={node}
                  tool={node.slug ? toolsBySlug[node.slug] : null}
                  toolMissing={!!node.slug && toolsBySlug[node.slug] === null}
                  chainLabel={node.chainId ? chainLabelById[node.chainId] : undefined}
                  selected={selectedId === node.id}
                  connectPending={linkingFromId === node.id}
                  readOnly={readOnly}
                  showRail={selectedId === node.id}
                  chainsPopoverOpen={chainsPopoverOpenId === node.id}
                  onSelect={handleNodeSelect}
                  onRemove={removeNode}
                  onTextChange={updateNodeText}
                  onChainsChange={updateNodeChains}
                  onOpenChains={(id) => setChainsPopoverOpenId(id)}
                  onCloseChains={() => setChainsPopoverOpenId(null)}
                  onPortPointerDown={handlePortPointerDown}
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
                  status={activeDragTool.status}
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

      <BlueprintShareDock
        blueprintId={blueprintId}
        isDraft={isDraft}
        title={title}
        nodes={nodes}
        edges={edges}
        toolsBySlug={toolsBySlug}
        readOnlyLayout={readOnlyLayout}
      />

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
      initialEdges={initialDraft.edges}
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
      initialEdges={blueprint.edges ?? []}
      initialSavedAt={blueprint.updated_at}
      readOnlyLayout={readOnlyLayout}
    />
  );
}