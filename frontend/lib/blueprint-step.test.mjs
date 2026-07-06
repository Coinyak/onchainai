import assert from "node:assert/strict";
import test from "node:test";
import { parseBlueprintStepInput, parseBlueprintStepsInput } from "./blueprint-step-core.mjs";

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

test("parseBlueprintStepsInput parses hash-prefixed numbers", () => {
  assert.deepEqual(parseBlueprintStepsInput("#1 #7"), [1, 7]);
  assert.deepEqual(parseBlueprintStepsInput("#3 #1"), [1, 3]);
});

test("parseBlueprintStepsInput parses comma-separated numbers", () => {
  assert.deepEqual(parseBlueprintStepsInput("1,7"), [1, 7]);
  assert.deepEqual(parseBlueprintStepsInput(" 1 , 7 "), [1, 7]);
});

test("parseBlueprintStepsInput deduplicates and sorts", () => {
  assert.deepEqual(parseBlueprintStepsInput("#7 #1 #7"), [1, 7]);
});

test("parseBlueprintStepsInput clamps to max step", () => {
  assert.deepEqual(parseBlueprintStepsInput("#150", 99), [99]);
});

test("parseBlueprintStepsInput caps at 8 numbers", () => {
  const result = parseBlueprintStepsInput("#1 #2 #3 #4 #5 #6 #7 #8 #9");
  assert.equal(result.length, 8);
  assert.deepEqual(result, [1, 2, 3, 4, 5, 6, 7, 8]);
});

test("parseBlueprintStepsInput clears on empty or invalid", () => {
  assert.deepEqual(parseBlueprintStepsInput(""), []);
  assert.deepEqual(parseBlueprintStepsInput("abc"), []);
  assert.deepEqual(parseBlueprintStepsInput("#0"), []);
});