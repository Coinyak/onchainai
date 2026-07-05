import assert from "node:assert/strict";
import test from "node:test";
import { parseBlueprintStepInput } from "./blueprint-step-core.mjs";

test("parseBlueprintStepInput accepts valid steps", () => {
  assert.equal(parseBlueprintStepInput("1"), 1);
  assert.equal(parseBlueprintStepInput(" 12 "), 12);
  assert.equal(parseBlueprintStepInput("99"), 99);
});

test("parseBlueprintStepInput clamps to max", () => {
  assert.equal(parseBlueprintStepInput("150", 99), 99);
});

test("parseBlueprintStepInput clears on empty or invalid", () => {
  assert.equal(parseBlueprintStepInput(""), undefined);
  assert.equal(parseBlueprintStepInput("0"), undefined);
  assert.equal(parseBlueprintStepInput("abc"), undefined);
});