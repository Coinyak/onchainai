import assert from "node:assert/strict";
import test from "node:test";
import {
  shouldAutoRegenSharePrompt,
  shouldShowStaleShareBanner,
} from "./blueprint-share-regen-core.mjs";

test("shouldAutoRegenSharePrompt when canvas changed and prompt pristine", () => {
  assert.equal(
    shouldAutoRegenSharePrompt({
      open: true,
      hasNodes: true,
      loading: false,
      baselineFingerprint: "fp:aaa",
      canvasFingerprint: "fp:bbb",
      isDirty: false,
    }),
    true,
  );
});

test("shouldAutoRegenSharePrompt blocks when panel closed or dirty", () => {
  assert.equal(
    shouldAutoRegenSharePrompt({
      open: false,
      hasNodes: true,
      loading: false,
      baselineFingerprint: "fp:aaa",
      canvasFingerprint: "fp:bbb",
      isDirty: false,
    }),
    false,
  );
  assert.equal(
    shouldAutoRegenSharePrompt({
      open: true,
      hasNodes: true,
      loading: false,
      baselineFingerprint: "fp:aaa",
      canvasFingerprint: "fp:bbb",
      isDirty: true,
    }),
    false,
  );
});

test("shouldShowStaleShareBanner only when dirty and stale", () => {
  assert.equal(shouldShowStaleShareBanner(true, true), true);
  assert.equal(shouldShowStaleShareBanner(true, false), false);
  assert.equal(shouldShowStaleShareBanner(false, true), false);
});