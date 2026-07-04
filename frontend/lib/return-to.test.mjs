import assert from "node:assert/strict";
import test from "node:test";
import { safeReturnTo } from "./return-to-guard.mjs";

test("safeReturnTo accepts same-origin paths", () => {
  assert.equal(safeReturnTo("/tools"), "/tools");
  assert.equal(safeReturnTo("/admin/tools?status=pending"), "/admin/tools?status=pending");
});

test("safeReturnTo rejects open redirects", () => {
  assert.equal(safeReturnTo("//evil.com"), null);
  assert.equal(safeReturnTo("https://evil.com"), null);
  assert.equal(safeReturnTo("/login:evil"), null);
  assert.equal(safeReturnTo(""), null);
  assert.equal(safeReturnTo(null), null);
});

test("safeReturnTo rejects backslash bypass (WHATWG normalizes \\ to / for special schemes)", () => {
  assert.equal(safeReturnTo("/\\evil.com"), null);
  assert.equal(safeReturnTo("/\\/evil.com"), null);
  assert.equal(safeReturnTo("/\\\\evil.com"), null);
});