# Agent Readiness Report

> Related docs: [AGENT_HARNESS](AGENT_HARNESS.md) | [BUILD_DEPLOY_RULES](BUILD_DEPLOY_RULES.md) | [AGENTS](../AGENTS.md)

Use the readiness report when preparing this repo for Codex, Claude Code, Droid, Cursor, Copilot, Grok, or other coding agents. It is a Droid-style status report, not a replacement for task-specific tests.

## Command

```bash
./scripts/agent-readiness-report.sh
```

The report prints Markdown with:

- applications discovered
- current readiness level
- 5-level progression table
- 9-pillar breakdown
- criteria-level pass/fail evidence
- prioritized action items
- local environment checks

## Model

The local model follows the same shape as Droid/Factory-style readiness:

- **Level 1: Functional Foundations** — basic build, test, docs, secrets hygiene.
- **Level 2: Documented Workflow** — LLM-wiki routing and topic docs.
- **Level 3: Reliable Automation** — executable gates, browser smoke, cargo-leptos, wasm target.
- **Level 4: Operational Safety** — RLS/security, redaction, tracing, deploy verification, CI.
- **Level 5: Autonomous Scale** — task discovery, agent review harness, product metrics, ownership/templates.

Pillars are: Agent Harness, Build System, Debugging & Observability, Development Environment, Documentation, Product & Experimentation, Security, Style & Validation, Task Discovery, and Testing.

## Status Meaning

- `READY`: core levels and local environment are clean.
- `READY WITH WARNINGS`: agents can work, but some higher-level criteria, QA, or local build paths are limited.
- `NOT READY`: fix blocking lower-level or environment failures before assigning risky UI/auth/routing work to agents.

Use `--json` for machine-readable output. `--strict` fails on `NOT READY`. CI uses `--strict-ci` (fails on any L1–L4 criterion miss; see `.github/workflows/ci.yml`).

## Operating Rule

When a readiness issue becomes recurring, prefer adding an executable check to `scripts/agent-readiness-report.sh` or `scripts/agent-harness-check.sh` instead of adding long prose to `AGENTS.md`.
