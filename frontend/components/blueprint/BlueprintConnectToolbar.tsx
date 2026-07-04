"use client";

import {
  BLUEPRINT_EDGE_COLORS,
  type BlueprintEdgeStyle,
} from "@/lib/blueprint-utils";

interface BlueprintConnectToolbarProps {
  connectMode: boolean;
  edgeStyle: BlueprintEdgeStyle;
  edgeColor: string;
  selectedEdgeId: string | null;
  readOnly: boolean;
  onToggleConnectMode: () => void;
  onEdgeStyleChange: (style: BlueprintEdgeStyle) => void;
  onEdgeColorChange: (color: string) => void;
  onDeleteEdge: () => void;
}

export function BlueprintConnectToolbar({
  connectMode,
  edgeStyle,
  edgeColor,
  selectedEdgeId,
  readOnly,
  onToggleConnectMode,
  onEdgeStyleChange,
  onEdgeColorChange,
  onDeleteEdge,
}: BlueprintConnectToolbarProps) {
  if (readOnly) return null;

  return (
    <div className="blueprint-connect-toolbar" data-testid="blueprint-connect-toolbar">
      <button
        type="button"
        className={`blueprint-toolbar-btn${connectMode ? " blueprint-toolbar-btn-active" : ""}`}
        data-testid="blueprint-connect-toggle"
        onClick={onToggleConnectMode}
        aria-pressed={connectMode}
      >
        {connectMode ? "Connecting..." : "Connect"}
      </button>
      <div className="blueprint-edge-style-group" role="group" aria-label="Line style">
        <button
          type="button"
          className={`blueprint-edge-style-btn${edgeStyle === "solid" ? " blueprint-edge-style-btn-active" : ""}`}
          onClick={() => onEdgeStyleChange("solid")}
          aria-pressed={edgeStyle === "solid"}
        >
          Solid
        </button>
        <button
          type="button"
          className={`blueprint-edge-style-btn${edgeStyle === "arrow" ? " blueprint-edge-style-btn-active" : ""}`}
          onClick={() => onEdgeStyleChange("arrow")}
          aria-pressed={edgeStyle === "arrow"}
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
            onClick={() => onEdgeColorChange(option.value)}
          />
        ))}
      </div>
      {selectedEdgeId && (
        <button
          type="button"
          className="blueprint-toolbar-btn blueprint-toolbar-btn-danger"
          data-testid="blueprint-delete-edge"
          onClick={onDeleteEdge}
        >
          Delete link
        </button>
      )}
    </div>
  );
}