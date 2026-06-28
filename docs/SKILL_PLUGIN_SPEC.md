# OnchainAI — Skill & Plugin 배포 스펙

> 작성일: 2026-06-29 · 상위 전략: [PRODUCT_ENHANCEMENT_SPEC.md](PRODUCT_ENHANCEMENT_SPEC.md) §J.
> 목적: OnchainAI를 MCP 서버 단독이 아니라 **Skill(노하우) + Plugin(원클릭 배포)**으로 패키징하는 구체 스펙.
> ⚠️ Skill/Plugin 포맷은 진화 중 — 구현 전 현재 Claude Code 플러그인 / Agent Skills 공식 문서로 필드 실측 확인(이 문서는 설계 의도 + 초안).

---

## 0. 3계층 요약

| 계층 | 산출물 | 역할 |
|---|---|---|
| MCP (있음) | `POST /mcp` | 런타임/데이터 — 툴 호출 |
| **Skill** (J1) | `SKILL.md` | 노하우 — 언제/어떻게 쓰고 결과를 해석할지 |
| **Plugin** (J2) | 플러그인 번들 | 배포 — MCP설정+Skill+커맨드 원클릭 설치 |

원칙: Skill은 MCP를 *전제*하고 "판단 규칙"을 더한다(설치 위험도·x402·official). Plugin은 그 둘 + 슬래시 커맨드를 묶어 config 복붙을 없앤다.

---

## 1. Skill 스펙 (J1)

### 1.1 메타데이터
- **name**: `onchainai-crypto-tools`
- **description** (트리거 좌우 — 구체적으로): "Find, vet, and install crypto/onchain tools (MCP servers, CLIs, SDKs, APIs, x402, AI-agent tools) via the OnchainAI directory. Use when the user needs an onchain capability and lacks a tool, asks 'what tool for X chain/task', or wants to install/compare crypto tools. Judges trust, install risk, and x402 payment before recommending."
- 위치(플러그인 내): `skills/onchainai-crypto-tools/SKILL.md`

### 1.2 SKILL.md 본문 초안

```markdown
---
name: onchainai-crypto-tools
description: Find, vet, and install crypto/onchain tools (MCP, CLI, SDK, API, x402, AI-agent) via the OnchainAI directory. Use when the user needs an onchain capability and lacks a tool, asks which tool fits a chain/task, or wants to install or compare crypto tools. Judges trust, install risk, and x402 payment before recommending.
---

# OnchainAI Crypto Tool Finder

Use the connected `onchainai` MCP server to discover and safely install crypto tools.

## When to use
- The user wants to do something onchain (swap, bridge, query a chain, pay via
  x402, run an agent) but doesn't have a tool for it.
- The user asks "what MCP/CLI/SDK for <chain or task>?" or "compare X and Y".
- The user wants install steps for Claude/Cursor.

## How to query (MCP tools)
- `search_tools(query, category?, chain?)` — primary discovery. Use the user's
  intent as `query`; pass `chain` (e.g. "base", "solana") and `category` when known.
- `get_tool_detail(slug)` — full metadata for one tool.
- `get_install_guide(slug, platform)` — platform ∈ {claude, cursor, generic}.
- `list_categories()` — when the user is browsing, not searching.

## Interpreting results — trust & safety (ALWAYS apply)
- `install_risk_level`:
  - `critical` → **DO NOT install or run.** Tell the user it's blocked pending review.
  - `high` → warn; do not paste raw shell wrappers; install only if the user trusts the source.
  - `medium`/`low` → proceed with the provided command.
- x402 / paid tools (`pricing = "x402"` or `x402_price` set): tell the user it
  charges on call and needs a connected agent wallet. Surface the price. Note
  whether `payment_verified`/`x402_endpoint_verified`/`price_verified` are true
  ("operator verified") or not ("not yet verified").
- Prefer `official`/`claimed` tools and higher trust/stars when several match.

## Installing
1. Pick the best low/medium-risk match (or the user's choice).
2. Call `get_install_guide(slug, platform)` for the user's agent.
3. Show the exact command / MCP config JSON and the steps. Never invent commands.

## Hard rules
- Never run or recommend running a `critical`-risk install command.
- Always state x402 cost + wallet requirement BEFORE the user calls a paid tool.
- Don't fabricate tools or install commands — only report what the MCP returns.
```

### 1.3 수용 기준
- 관련 상황(온체인 작업 + 툴 부재)에서 스킬이 트리거되어 `search_tools` 호출.
- critical 위험 차단·x402 고지·official 우선 규칙을 응답에서 준수.
- Claude 앱 / Claude Code / API(코드실행) 모두에서 로드 가능.

---

## 2. Plugin 스펙 (J2)

### 2.1 디렉터리 구조 (초안)
```
onchainai-plugin/
├── .claude-plugin/
│   └── plugin.json
├── .mcp.json                       # MCP 서버 자동 연결
├── commands/
│   └── find-tool.md                # /find-tool 슬래시 커맨드
└── skills/
    └── onchainai-crypto-tools/
        └── SKILL.md                # §1.2
```

### 2.2 `.claude-plugin/plugin.json` (초안)
```json
{
  "name": "onchainai",
  "version": "0.1.0",
  "description": "Discover, vet, and install crypto MCP/CLI/SDK/API/x402/AI-agent tools via the OnchainAI directory.",
  "author": { "name": "OnchainAI", "url": "https://www.onchain-ai.xyz" },
  "homepage": "https://www.onchain-ai.xyz",
  "keywords": ["crypto", "web3", "mcp", "x402", "onchain", "agent-tools"]
}
```

### 2.3 `.mcp.json` (원클릭 MCP 연결)
```json
{
  "mcpServers": {
    "onchainai": {
      "command": "npx",
      "args": ["mcp-remote", "https://www.onchain-ai.xyz/mcp"]
    }
  }
}
```
> 기존 install guide가 안내하는 엔드포인트와 동일([mcp.rs](../src/server/mcp.rs) cursor 분기, [UI_UX_DESIGN.md](UI_UX_DESIGN.md) "Connect MCP" 카드).

### 2.4 `commands/find-tool.md` (슬래시 커맨드)
```markdown
---
description: Find a crypto MCP/CLI/SDK/x402 tool via OnchainAI
argument-hint: <what you need, e.g. "bridge USDC to Base">
---
Use the `onchainai` MCP `search_tools` to find tools matching: $ARGUMENTS

Summarize the top 3 results with: name, what it does, chains, type, install
risk, and x402/paid status. Then offer install steps via `get_install_guide`
for the user's agent. Never recommend a critical-risk tool.
```

### 2.5 마켓플레이스 등재 — `.claude-plugin/marketplace.json` (배포)
```json
{
  "name": "onchainai",
  "owner": { "name": "OnchainAI", "url": "https://www.onchain-ai.xyz" },
  "plugins": [
    {
      "name": "onchainai",
      "source": "./",
      "description": "Crypto tool directory: find, vet, install MCP/CLI/SDK/x402 tools."
    }
  ]
}
```
> A5(외부 레지스트리 등재)와 함께 배포 채널. 호스팅: 별도 repo(예: `onchainai/plugin`) 또는 메인 repo 하위 디렉터리.

### 2.6 수용 기준
- 플러그인 설치 시: `onchainai` MCP 자동 연결 + Skill 로드 + `/find-tool` 동작.
- config 수동 복붙 없이 발견→설치 동선 완결.

---

## 3. J3 — 발견한 툴을 Skill/Plugin으로 export (차별화)

- **컬렉션 → 플러그인 매니페스트 생성기**: 큐레이션 컬렉션(G1, 예 "Base 에이전트 스타터킷")을 입력하면 위 구조의 플러그인 번들(.mcp.json에 묶음 MCP들 + 컬렉션 설명 Skill)을 생성 → 한 방 설치.
- **카탈로그가 skill/plugin도 1급 시민**: `type=skill` 이미 존재, Smithery는 skills 보유(B2). `type` 패싯에 `plugin` 추가(C4 WebSocket과 동일, 마이그레이션 불필요).
- 수용 기준: 컬렉션→번들 생성 1건, `type=skill|plugin` 필터 동작.

---

## 4. 구현 순서
1. **J1 Skill** — `SKILL.md` 작성 + Claude/Claude Code에서 트리거·안전규칙 검증. (가장 싸고 즉효)
2. **J2 Plugin** — 위 구조로 번들 + 로컬 설치 테스트(MCP 자동연결·커맨드).
3. **마켓플레이스 등재** + A5 외부 레지스트리 등재.
4. **J3** — 컬렉션 export 생성기 + type=plugin 패싯.

## 5. 주의
- 플러그인/스킬 스키마 필드는 현재 공식 문서로 실측(이 초안은 의도 표현). 
- 보안: 플러그인이 자동 연결하는 MCP는 우리 자신의 엔드포인트뿐(제3자 임의 실행 없음). 슬래시 커맨드는 critical 위험 툴 권유 금지 규칙 포함.
