# Public Dashboard And Toolkit Implementation Plan

## Goal

Ship the first usable version of two connected product surfaces:

- A public, no-login `/dashboard` that turns the crawled tool database into an at-a-glance market map.
- A signed-in `/toolkit` that treats saved bookmarks as the user's personal tool cart for later install, agent setup, and export.

The MCP server should expose the same public dashboard snapshot so agents can inspect the directory state without scraping the website.

## Scope

- Add a public dashboard snapshot server function backed by approved public tools only.
- Add dashboard aggregate metrics, bucket links, and bounded tool lists.
- Add a `get_dashboard_snapshot` MCP tool with bounded output.
- Add a toolkit server payload for the current user's bookmarked tools.
- Add Markdown and JSON toolkit exports from sanitized public tool rows.
- Add routes, top navigation, and page components for `/dashboard` and `/toolkit`.
- Update bookmark UI language from generic bookmarking to Toolkit language.
- Include the detailed product spec and benchmark document in this branch.

## Non-Goals

- No payment execution, custody proxy, facilitator gateway, or x402 fund movement.
- No anonymous persistent toolkit. Anonymous users can browse the dashboard and are asked to sign in before saving tools.
- No schema migration for a separate cart table in this version; bookmarks already model saved tools safely.
- No admin-only metrics or private crawler data on the public dashboard.

## Validation

- Add contract tests before implementation for dashboard limits, dashboard filter links, sanitized toolkit export, and MCP tool registration.
- Run targeted tests to confirm the new tests fail before production code.
- Run `cargo fmt --check`, `cargo test --features ssr`, `cargo clippy --features ssr -- -W clippy::all`, and `git diff --check`.
- Build and smoke the SSR app, then inspect desktop and mobile screenshots for the new routes.
