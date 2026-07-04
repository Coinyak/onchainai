# OnchainAI Documentation Index

> Obsidian-compatible knowledge base. All docs are cross-linked markdown.
> AI agents: read AGENTS.md first, then relevant docs below.

## Architecture

- [[MVP_DESIGN]] — Full architecture: Rust single binary, DB schema, crawler, MCP server, auth, admin panel, build order
- [[SECURITY]] — Security design: 3-way auth, SIWX, RLS policies, web security headers, rate limiting, admin access control

## Design

- [[UI_UX_DESIGN]] — Full UI spec: pages, components, sidebar, preview panel, bottom sheet, mobile, admin dashboard
- [[../DESIGN]] — Stitch DESIGN.md: design tokens (colors, typography, components) for AI UI generation

## Product & Launch

- [[REVENUE_FORECAST]] — 최종 수익 전망: 무료 발견 + 에이전트 신뢰 + 프로바이더 B2B (어트리뷰션 P&L $0)
- [[CONNECT]] — Connect any MCP client, install the Claude Code plugin/skill (user-facing)
- [[LAUNCH_READINESS_SPEC]] — GitHub 공개 전환 체크리스트, 온보딩 표면, 등재 채널, x402 검증 잡, 어댑션 계측
- [[X402_MONETIZATION_SPEC]] — x402 수익·제품 정본 (가격, 무료 티어, 금지, DoD)
- [[X402_OPEN_LISTING_SPEC]] — x402 self-serve open listing + K2 check_endpoint_health premium (facilitator settle, no-custody)
- [[X402_ROADMAP]] — x402 로드맵 (living plan: 페이즈 체크리스트·스프린트·KPI; **자문용, 변동 가능**)
- [[X402_REFERRAL_SPEC]] — x402 레퍼럴/어트리뷰션 설계 (no-custody 원칙)
- [[MCP_X402_MONETIZATION_SPEC]] — MCP premium Axis B x402 (compare_tools/export_toolkit)
- [[superpowers/specs/2026-07-04-free-tier-guardian-spec]] — 영구 무료 티어 정책 (웹·MCP·compare_tools·SEO `/x402` 허브)
- [[SKILL_PLUGIN_SPEC]] — Skill + Plugin 패키징 스펙 (`plugin/onchainai/` 레이아웃)

## Operations & Deploy

- [[OPERATOR_GUIDE]] — Operator/admin capabilities (Korean): what each role can do without code changes
- [[BUILD_DEPLOY_RULES]] — Build coherence, smoke gates, Railway Dockerfile deploy

## Agent Configuration

- [[../AGENTS.md]] — Short routing entry point for all coding agents
- [[AGENT_HARNESS]] — Wiki-style agent workflow and executable UI/auth/routing gates
- [[MCP_AGENT_WORKFLOW]] — Vercel/Railway/GitHub/onchainai MCP routing and deploy observability
- [[MULTI_AGENT_COORDINATION]] — Five-subagent roster, DAG, verification matrix
- [[handoff-packet-template]] — Copy-paste handoff between subagents
- [[AGENT_READINESS_REPORT]] — Environment readiness report for coding agents
- [[../CLAUDE.md]] — Claude Code entry point (imports AGENTS.md)

## External References

- [AGENTS.md spec](https://agents.md/) — Open standard, Linux Foundation
- [Harness Engineering](https://martinfowler.com/articles/harness-engineering.html) — Martin Fowler / Thoughtworks
- [Stitch DESIGN.md](https://stitch.withgoogle.com/docs/design-md/overview) — Google Stitch spec
- [SIWX (x402 V2)](https://docs.x402.org/extensions/sign-in-with-x) — CAIP-122 wallet auth
- [Supabase RLS](https://supabase.com/docs/guides/database/postgres/row-level-security) — Row Level Security
