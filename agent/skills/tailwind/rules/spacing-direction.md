---
title: Spacing Direction — Prefer Bottom Flow
impact: HIGH
tags: spacing, margin, padding, gap
---

**Rule**: Prefer `mb-*`, `pb-*`, or `gap` for vertical spacing when they fit the component structure. Use `mt-*` or `pt-*` when top spacing is the correct semantic choice, such as first-child offsets, hero sections, or inset cards.

### Incorrect

```tsx
<div className="mt-4 pt-4">
  <h2 className="mt-6">Title</h2>
  <p className="mt-2">Content</p>
</div>
```

### Correct

```tsx
<div className="mb-4 pb-4">
  <h2 className="mb-2">Title</h2>
  <p>Content</p>
</div>

<!-- Or use gap on parent -->
<div className="flex flex-col gap-4">
  <h2>Title</h2>
  <p>Content</p>
</div>
```
