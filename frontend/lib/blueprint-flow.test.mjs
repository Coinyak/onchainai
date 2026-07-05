import assert from "node:assert/strict";
import test from "node:test";
import { buildFlowSection } from "./blueprint-flow-core.mjs";

test("buildFlowSection keeps linear path on one line", () => {
  const nodes = [
    { id: "a", kind: "tool", slug: "alpha" },
    { id: "b", kind: "tool", slug: "beta" },
    { id: "c", kind: "tool", slug: "gamma" },
  ];
  const edges = [
    { id: "e1", fromId: "a", toId: "b" },
    { id: "e2", fromId: "b", toId: "c" },
  ];

  assert.equal(buildFlowSection(nodes, edges), "- alpha → beta → gamma");
});

test("buildFlowSection splits at branch points", () => {
  const nodes = [
    { id: "hub", kind: "tool", slug: "gateway" },
    { id: "base", kind: "chain", chainId: "base" },
    { id: "bnb", kind: "chain", chainId: "bsc" },
  ];
  const edges = [
    { id: "e1", fromId: "hub", toId: "base" },
    { id: "e2", fromId: "hub", toId: "bnb", label: "swap" },
  ];

  const flow = buildFlowSection(nodes, edges);
  const lines = flow.split("\n");

  assert.equal(lines.length, 2);
  assert.match(flow, /gateway → chain: base/);
  assert.match(flow, /gateway →\(swap\) chain: bsc/);
});

test("buildFlowSection tie-breaks equal destination labels by node id", () => {
  const nodes = [
    { id: "hub", kind: "tool", slug: "gateway" },
    { id: "aaa", kind: "chain", chainId: "base" },
    { id: "bbb", kind: "chain", chainId: "base" },
  ];
  const edges = [
    { id: "e2", fromId: "hub", toId: "bbb" },
    { id: "e1", fromId: "hub", toId: "aaa" },
  ];

  const flow = buildFlowSection(nodes, edges);
  const idxAaa = flow.indexOf("chain: base");
  const idxBbb = flow.lastIndexOf("chain: base");
  assert.ok(idxAaa >= 0 && idxBbb > idxAaa);
});