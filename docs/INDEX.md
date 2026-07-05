# OnchainAI Documentation Index

> Cross-linked Markdown knowledge base — plain relative links that render on GitHub
> and in any editor (no Obsidian or plugin required).
> AI agents: read [AGENTS.md](../AGENTS.md) first for routing, then the relevant doc below.

## Architecture

- [MVP_DESIGN](MVP_DESIGN.md) — Full architecture: Rust single binary, DB schema, crawler, MCP server, auth, admin panel, build order
- [SECURITY](SECURITY.md) — Security design: 3-way auth, SIWX, RLS policies, web security headers, rate limiting, admin access control
- [TOOL_DISCOVERY](TOOL_DISCOVERY.md) — 자동 발견(Discovery) 전략 + 시드 후보: 소스 연결 + LLM 보강·갭분석·신규탐지 레이어
- [SEED_DATA](SEED_DATA.md) — Seed & fixture data spec: 로컬 개발 시드 + 자동 테스트 픽스처 (list/category/comment/admin-queue 화면 검증용)

## Design

- [UI_UX_DESIGN](UI_UX_DESIGN.md) — Full UI spec: pages, components, sidebar, preview panel, bottom sheet, mobile, admin dashboard
- [DESIGN](../DESIGN.md) — Stitch DESIGN.md: design tokens (colors, typography, components) for AI UI generation

## Product & Launch

- [REVENUE_FORECAST](REVENUE_FORECAST.md) — 최종 수익 전망: 무료 발견 + 에이전트 신뢰 + 프로바이더 B2B (어트리뷰션 P&L $0)
- [CONNECT](CONNECT.md) — Connect any MCP client, install the Claude Code plugin/skill (user-facing)
- [LAUNCH_READINESS_SPEC](LAUNCH_READINESS_SPEC.md) — GitHub 공개 전환 체크리스트, 온보딩 표면, 등재 채널, x402 검증 잡, 어댑션 계측
- [PRODUCT_ENHANCEMENT_SPEC](PRODUCT_ENHANCEMENT_SPEC.md) — 고도화 스펙: 코드베이스 진단 → 우선순위 개선(기능·MCP·UI·견고화), 근거(파일:라인)→목표→작업→수용기준
- [X402_MONETIZATION_SPEC](X402_MONETIZATION_SPEC.md) — x402 수익·제품 정본 (가격, 무료 티어, 금지, DoD)
- [X402_OPEN_LISTING_SPEC](X402_OPEN_LISTING_SPEC.md) — x402 self-serve open listing + K2 check_endpoint_health premium (facilitator settle, no-custody)
- [X402_ROADMAP](X402_ROADMAP.md) — x402 로드맵 (living plan: 페이즈 체크리스트·스프린트·KPI; **자문용, 변동 가능**)
- [X402_REFERRAL_SPEC](X402_REFERRAL_SPEC.md) — x402 레퍼럴/어트리뷰션 설계 (no-custody 원칙)
- [MCP_X402_MONETIZATION_SPEC](MCP_X402_MONETIZATION_SPEC.md) — MCP premium Axis B x402 (compare_tools/export_toolkit) — **Superseded**, X402_MONETIZATION_SPEC가 정본
- [free-tier-guardian-spec](superpowers/specs/2026-07-04-free-tier-guardian-spec.md) — 영구 무료 티어 정책 (웹·MCP·compare_tools·SEO `/x402` 허브)
- [SKILL_PLUGIN_SPEC](SKILL_PLUGIN_SPEC.md) — Skill + Plugin 패키징 스펙 (`plugin/onchainai/` 레이아웃)
- [listings/](listings/) — 외부 디렉토리 등재 카피·제출 폼 (awesome-crypto-mcp-servers 엔트리, directory-forms)

## Operations & Deploy

- [OPERATOR_GUIDE](OPERATOR_GUIDE.md) — Operator/admin capabilities (Korean): what each role can do without code changes
- [FEATURED_CARDS](FEATURED_CARDS.md) — 하이라이트 캐러셀(프로모 카드) 승격/내림 오퍼레이터 플레이북 + 이미지 소싱
- [BUILD_DEPLOY_RULES](BUILD_DEPLOY_RULES.md) — Build coherence, smoke gates, Railway Dockerfile deploy
- [BRANCH_PROTECTION](BRANCH_PROTECTION.md) — main 머지 전 필수 `ci-success` 집계 체크 구성
- [DISK_MAINTENANCE](DISK_MAINTENANCE.md) — 디스크 위생: 멀티-GB 링커 스냅샷 자동 정리 (macOS 스케줄러)

## Agent Configuration

- [AGENTS.md](../AGENTS.md) — Short routing entry point for all coding agents
- [AGENT_HARNESS](AGENT_HARNESS.md) — Wiki-style agent workflow and executable UI/auth/routing gates
- [MCP_AGENT_WORKFLOW](MCP_AGENT_WORKFLOW.md) — Vercel/Railway/GitHub/onchainai MCP routing and deploy observability
- [MULTI_AGENT_COORDINATION](MULTI_AGENT_COORDINATION.md) — Five-subagent roster, DAG, verification matrix
- [handoff-packet-template](handoff-packet-template.md) — Copy-paste handoff between subagents
- [VERIFICATION](VERIFICATION.md) — 구현 검증 매트릭스 (에이전트 자가 검증): 항목별 기계 확인, 실행기 `scripts/spec-verify.sh`
- [AGENT_READINESS_REPORT](AGENT_READINESS_REPORT.md) — Environment readiness report for coding agents
- [CLAUDE.md](../CLAUDE.md) — Claude Code entry point (imports AGENTS.md)

## Archive

- [archive/README](archive/README.md) — 완료된 일회성 스펙·작업 지시서 보관소 (현행 규칙 아님; 에이전트는 작업 근거로 사용 금지)

## External References

- [AGENTS.md spec](https://agents.md/) — Open standard, Linux Foundation
- [Harness Engineering](https://martinfowler.com/articles/harness-engineering.html) — Martin Fowler / Thoughtworks
- [Stitch DESIGN.md](https://stitch.withgoogle.com/docs/design-md/overview) — Google Stitch spec
- [SIWX (x402 V2)](https://docs.x402.org/extensions/sign-in-with-x) — CAIP-122 wallet auth
- [Supabase RLS](https://supabase.com/docs/guides/database/postgres/row-level-security) — Row Level Security
