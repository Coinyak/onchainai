import assert from "node:assert/strict";
import test from "node:test";
import { blueprintCanvasFingerprint } from "./blueprint-share-fingerprint-core.mjs";

const baseNodes = [
  { id: "a", kind: "tool", slug: "onchainai", step: 1 },
  { id: "b", kind: "note", text: "hello" },
];

test("blueprintCanvasFingerprint is deterministic and step-sensitive", () => {
  const fp1 = blueprintCanvasFingerprint("Agent stack", baseNodes, []);
  assert.match(fp1, /^fp:[0-9a-f]{8}$/);
  assert.equal(fp1, blueprintCanvasFingerprint("Agent stack", baseNodes, []));

  const withStep2 = [
    { id: "a", kind: "tool", slug: "onchainai", step: 2 },
    { id: "b", kind: "note", text: "hello" },
  ];
  assert.notEqual(fp1, blueprintCanvasFingerprint("Agent stack", withStep2, []));
});

test("blueprintCanvasFingerprint ignores node order", () => {
  const reversed = [...baseNodes].reverse();
  assert.equal(
    blueprintCanvasFingerprint("T", baseNodes, []),
    blueprintCanvasFingerprint("T", reversed, []),
  );
});

test("blueprintCanvasFingerprint changes when note text changes", () => {
  const fp1 = blueprintCanvasFingerprint("T", baseNodes, []);
  const editedNote = [
    { id: "a", kind: "tool", slug: "onchainai", step: 1 },
    { id: "b", kind: "note", text: "updated" },
  ];
  assert.notEqual(fp1, blueprintCanvasFingerprint("T", editedNote, []));
});

test("blueprintCanvasFingerprint changes when edge style or label changes", () => {
  const nodes = [{ id: "a", kind: "tool", slug: "onchainai" }];
  const edgeA = [
    {
      id: "e1",
      fromId: "a",
      toId: "a",
      style: "solid",
      color: "#000",
      label: "first",
    },
  ];
  const edgeB = [
    {
      id: "e1",
      fromId: "a",
      toId: "a",
      style: "dashed",
      color: "#000",
      dashed: true,
      label: "second",
    },
  ];
  assert.notEqual(
    blueprintCanvasFingerprint("T", nodes, edgeA),
    blueprintCanvasFingerprint("T", nodes, edgeB),
  );
});