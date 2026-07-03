"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import type { Tool } from "@/lib/api";
import { ToolDetail } from "@/components/tools/ToolDetail";
import { PreviewPanelContent } from "@/components/tools/PreviewPanelContent";
import { PreviewActionBar } from "@/components/tools/PreviewActionBar";

const DRAG_EXPAND_THRESHOLD = 48;
const DRAG_COLLAPSE_THRESHOLD = 48;
const DRAG_CLOSE_THRESHOLD = 96;

interface BottomSheetProps {
  tool: Tool;
  closeHref: string;
  fullPageHref: string;
  commentCount?: number;
  addMode?: boolean;
  addMcpQueryBase?: string;
  compareReturnHref?: string;
}

function getFocusableElements(root: HTMLElement): HTMLElement[] {
  return Array.from(
    root.querySelectorAll<HTMLElement>(
      'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  ).filter((el) => !el.hasAttribute("disabled") && el.offsetParent !== null);
}

export function BottomSheet({
  tool,
  closeHref,
  fullPageHref,
  commentCount = 0,
  addMode = false,
  addMcpQueryBase = "",
  compareReturnHref = "",
}: BottomSheetProps) {
  const router = useRouter();
  const sheetRef = useRef<HTMLDivElement>(null);
  const dragStartY = useRef(0);
  const dragging = useRef(false);
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = "";
    };
  }, []);

  useEffect(() => {
    function onKeyDown(ev: KeyboardEvent) {
      if (ev.key === "Escape") {
        ev.stopPropagation();
        router.push(closeHref);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [closeHref, router]);

  useEffect(() => {
    if (!expanded || !sheetRef.current) return;

    const sheet = sheetRef.current;
    const focusable = getFocusableElements(sheet);
    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    first.focus({ preventScroll: true });

    function onTabKeyDown(ev: KeyboardEvent) {
      if (ev.key !== "Tab") return;
      const active = document.activeElement;
      if (ev.shiftKey) {
        if (active === first) {
          ev.preventDefault();
          last.focus({ preventScroll: true });
        }
      } else if (active === last) {
        ev.preventDefault();
        first.focus({ preventScroll: true });
      }
    }

    sheet.addEventListener("keydown", onTabKeyDown);
    return () => sheet.removeEventListener("keydown", onTabKeyDown);
  }, [expanded]);

  const toggleExpanded = useCallback(() => {
    setExpanded((value) => !value);
  }, []);

  const finishDrag = useCallback(
    (deltaY: number) => {
      if (!expanded) {
        if (deltaY < -DRAG_EXPAND_THRESHOLD) {
          setExpanded(true);
        } else if (deltaY > DRAG_CLOSE_THRESHOLD) {
          router.push(closeHref);
        }
        return;
      }

      if (deltaY > DRAG_COLLAPSE_THRESHOLD) {
        setExpanded(false);
      }
    },
    [closeHref, expanded, router],
  );

  function onHandlePointerDown(ev: React.PointerEvent<HTMLButtonElement>) {
    dragging.current = true;
    dragStartY.current = ev.clientY;
    ev.currentTarget.setPointerCapture(ev.pointerId);
  }

  function onHandlePointerMove(ev: React.PointerEvent<HTMLButtonElement>) {
    if (!dragging.current) return;
    ev.preventDefault();
  }

  function onHandlePointerUp(ev: React.PointerEvent<HTMLButtonElement>) {
    if (!dragging.current) return;
    dragging.current = false;
    const deltaY = ev.clientY - dragStartY.current;
    if (ev.currentTarget.hasPointerCapture(ev.pointerId)) {
      ev.currentTarget.releasePointerCapture(ev.pointerId);
    }
    if (Math.abs(deltaY) < 8) {
      toggleExpanded();
      return;
    }
    finishDrag(deltaY);
  }

  function onHandlePointerCancel(ev: React.PointerEvent<HTMLButtonElement>) {
    dragging.current = false;
    if (ev.currentTarget.hasPointerCapture(ev.pointerId)) {
      ev.currentTarget.releasePointerCapture(ev.pointerId);
    }
  }

  return (
    <>
      <Link href={closeHref} scroll={false} className="bottom-sheet-backdrop" aria-label="Close preview">
        <span className="sr-only">Close</span>
      </Link>
      <div
        ref={sheetRef}
        className={expanded ? "bottom-sheet bottom-sheet-full" : "bottom-sheet"}
        role="dialog"
        aria-modal="true"
        aria-label="Tool preview"
        data-testid="preview-bottom-sheet"
      >
        <button
          type="button"
          className="bottom-sheet-handle"
          aria-label={expanded ? "Tap to collapse" : "Tap to expand"}
          onPointerDown={onHandlePointerDown}
          onPointerMove={onHandlePointerMove}
          onPointerUp={onHandlePointerUp}
          onPointerCancel={onHandlePointerCancel}
        >
          <span className="bottom-sheet-handle-bar" aria-hidden="true" />
        </button>
        <div className="bottom-sheet-body">
          {addMode ? (
            <ToolDetail
              tool={tool}
              compact
              commentCount={commentCount}
              addMode={addMode}
              addMcpQueryBase={addMcpQueryBase}
              compareReturnHref={compareReturnHref}
            />
          ) : (
            <PreviewPanelContent
              key={tool.slug}
              tool={tool}
              closeHref={closeHref}
              fullPageHref={fullPageHref}
              commentCount={commentCount}
            />
          )}
          {!addMode && (
            <Link href={fullPageHref} className="bottom-sheet-view-full">
              View full page
            </Link>
          )}
        </div>
        {!addMode && (
          <PreviewActionBar
            key={`${tool.slug}-actions`}
            tool={tool}
            fullPageHref={fullPageHref}
            addMcpQueryBase={addMcpQueryBase}
          />
        )}
      </div>
    </>
  );
}