# Handoff Packet Template

> Copy this block between subagents. Coordinator merges packets before user handoff.
> Related: [MULTI_AGENT_COORDINATION](MULTI_AGENT_COORDINATION.md)

## Template

```markdown
## Handoff Packet

- **FROM:** <agent role>
- **TO:** <next agent role>
- **TASK_ID:** <short slug>
- **STATUS:** done | blocked | needs-review

### Files touched
- `path/to/file`

### Contract
- API shape / types / env var *names* (no secret values)
- Routes or `data-testid` changes

### Verification run
- `command` → PASS | FAIL (exit code)
- MCP queries (read-only): <server> — <what was checked>

### Open risks
- <blocker or follow-up>

### Next owner
- <role> — <one-line task>
```

## Example: Data → Backend

```markdown
## Handoff Packet

- **FROM:** Data & Schema
- **TO:** Backend Core
- **TASK_ID:** featured-cards-v2
- **STATUS:** done

### Files touched
- `migrations/027_featured_cards.sql`

### Contract
- Table `featured_cards`: `id`, `tool_id`, `sort_order`, `active`, `created_at`
- Env: none new
- Run `sqlx prepare` after migration apply

### Verification run
- `sqlx migrate run` → PASS
- `cargo sqlx prepare` → PASS

### Open risks
- None

### Next owner
- Backend Core — add `/api/v2/admin/featured-cards` CRUD handlers
```

## Example: Backend → Frontend

```markdown
## Handoff Packet

- **FROM:** Backend Core
- **TO:** Frontend Surface
- **TASK_ID:** featured-cards-v2
- **STATUS:** done

### Files touched
- `src/server/api_v2/admin_featured.rs`

### Contract
- `GET /api/v2/admin/featured-cards` → `{ items: FeaturedCard[] }`
- `FeaturedCard`: `{ id, tool_id, tool_name, sort_order, active }`
- Auth: admin session required (cookie)

### Verification run
- `cargo test featured_cards` → PASS
- Railway MCP — runtime log scan (read-only) → no new 5xx

### Open risks
- None

### Next owner
- Frontend Surface — admin UI list + reorder; use `frontend/lib/api.ts`
```