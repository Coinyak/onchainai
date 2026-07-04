import assert from "node:assert/strict";
import test from "node:test";

// Compiled-free import: duplicate safeReturnTo rules for unit coverage.
function safeReturnTo(raw) {
  if (!raw?.trim()) return null;
  const path = raw.trim();
  if (!path.startsWith("/") || path.startsWith("//")) return null;
  const [pathname] = path.split(/[?#]/, 1);
  if (!pathname || pathname.includes(":")) return null;
  return path;
}

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