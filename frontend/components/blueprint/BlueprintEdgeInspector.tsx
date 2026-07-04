"use client";

import {
  BLUEPRINT_EDGE_COLORS,
  type BlueprintEdgeStyle,
} from "@/lib/blueprint-utils";

interface BlueprintEdgeInspectorProps {
  visible: boolean;
  edgeStyle: BlueprintEdgeStyle;
  edgeColor: string;
  selectedEdgeId: string | null;
  readOnly: boolean;
  onStyleChange: (style: BlueprintEdgeStyle) => void;
  onColorChange: (color: string) => void;
  onDeleteEdge: () => void;
}

export function BlueprintEdgeInspector({
  visible,
  edgeStyle,
  edgeColor,
  selectedEdgeId,
  readOnly,
  onStyleChange,
  onColorChange,
  onDeleteEdge,
}: BlueprintEdgeInspectorProps) {
  return (
    <div
      className="blueprint-edge-inspector"
      data-testid="blueprint-edge-inspector"
      style={{
        visibility: visible ? "visible" : "hidden",
        pointerEvents: visible ? "auto" : "none",
        minHeight: 36,
        display: "flex",
        alignItems: "center",
        gap: 8,
        flexWrap: "wrap",
      }}
      aria-hidden={!visible}
    >
      <div className="blueprint-edge-style-group" role="group" aria-label="Line style">
        <button
          type="button"
          className={`blueprint-edge-style-btn${edgeStyle === "solid" ? " blueprint-edge-style-btn-active" : ""}`}
          onClick={() => onStyleChange("solid")}
          aria-pressed={edgeStyle === "solid"}
          disabled={readOnly}
        >
          Solid
        </button>
        <button
          type="button"
          className={`blueprint-edge-style-btn${edgeStyle === "arrow" ? " blueprint-edge-style-btn-active" : ""}`}
          onClick={() => onStyleChange("arrow")}
          aria-pressed={edgeStyle === "arrow"}
          disabled={readOnly}
        >
          Arrow
        </button>
      </div>
      <div className="blueprint-edge-color-group" role="group" aria-label="Line color">
        {BLUEPRINT_EDGE_COLORS.map((option) => (
          <button
            key={option.id}
            type="button"
            className={`blueprint-edge-color-btn${edgeColor === option.value ? " blueprint-edge-color-btn-active" : ""}`}
            style={{ backgroundColor: option.value }}
            aria-label={option.label}
            aria-pressed={edgeColor === option.value}
            onClick={() => onColorChange(option.value)}
            disabled={readOnly}
          />
        ))}
      </div>
      {selectedEdgeId && (
        <button
          type="button"
          className="blueprint-toolbar-btn blueprint-toolbar-btn-danger"
          data-testid="blueprint-delete-edge"
          onClick={onDeleteEdge}
          disabled={readOnly}
        >
          Delete link
        </button>
      )}
    </div>
  );
}