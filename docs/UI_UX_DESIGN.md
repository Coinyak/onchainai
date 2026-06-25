# OnchainAI UI/UX 설계

> 관련 문서: [[INDEX]] | [[MVP_DESIGN]] | [[SECURITY]] | [[../DESIGN]] | [[../AGENTS.md]]
>
> 작성: 2026-06-25. 사용자 요구사항 + 디렉토리 사이트 레퍼런스 분석 기반.

---

> UI 표시 텍스트는 **기본 영어** (글로벌 대상). 설계 문서 설명은 한글 유지.

---

## 0. 디자인 원칙

**키워드**: 깔끔, 세련됨, 가시성, 분류

- **라이트 모드** (다크 모드 안 함)
- **이모지 금지** — 대신: 공식 로고 이미지, 단색 SVG 아이콘, 색상 텍스트, 적절한 볼드체
- **정보 밀집** — 하지만 가시성 유지, 여백으로 호흡
- **분류 우선** — 탭/필터/카테고리로 명확히 구분
- **외부 링크 통합** — 유명 스킬/프롬프트/GitHub 링크 노출
- **기본 언어: 영어** (글로벌)

---

## 1. 레퍼런스 사이트

| 사이트 | 참고할 점 |
|---|---|
| **AlternativeTo** | 리스트 + 인라인 펼치기, 대안 추천, 별점 |
| **Product Hunt** | 깔끔한 카드 레이아웃, upvote, 댓글 |
| **GitHub** | stars, 최근 활동, 언어 배지, README 미리보기 |
| **Hacker News** | 정보 밀집 리스트, 댓글 우선 |
| **npm** | 패키지 카드, install 명령 복사, 주간 다운로드 |
| **Smithery** | MCP 서버 카드, 사용량 표시, "Add to toolbox" |

---

## 2. 페이지 구조

### 글로벌 네비게이션 (모든 페이지 상단)

```
┌────────────────────────────────────────────────────────────────┐
│  OnchainAI                                      [Submit]  [GitHub] │
└────────────────────────────────────────────────────────────────┘
```

> 상단 네비게이션은 로고 + Submit + GitHub 링크만.
> **sticky**: 스크롤 시 상단에 고정 (`position: sticky; top: 0; z-index: 100`).
> 배경: #FFFFFF + 하단 1px #E5E5E5 보더 (구분선).
> 탭/필터는 아래 소개 섹션 이후 왼쪽 사이드바로 이동 (3축 필터 구조).

### 홈페이지 (`/`)

상단 네비게이션 → 소개+검색+커뮤니티 등록 유도 카드 → 왼쪽 사이드바 필터 + 리스트.
Smithery 소개 카드 + Product Hunt 세로 리스트 + AlternativeTo 사이드바 필터 결합.

```
┌────────────────────────────────────────────────────────────────┐
│  OnchainAI                                      [Submit] [GitHub] │  ← 네비게이션
└────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────┐
│                                                                │  ← 소개 + 검색 + 등록 유도
│  OnchainAI — Crypto tools, unified.                            │
│                                                                │
│  Discover, install, and share crypto MCP, CLI, SDK, API,       │
│  x402, RWA, and AI agent tools — all in one place.             │
│  Search as a human, or let your agent search via MCP.          │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Search: asset tracking, trading, DeFi, chain name...     │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                │
│  ┌────────────────────────┐  ┌────────────────────────┐   │
│  │ Submit a Tool           │  │ Connect via MCP         │   │
│  │                        │  │                        │   │
│  │ List your MCP, CLI,     │  │ Connect your agent to   │   │
│  │ SDK, or API. Free,      │  │ OnchainAI MCP and       │   │
│  │ with verification badge.│  │ search 1,342 tools      │   │
│  │ x402 tools: pay per     │  │ instantly.              │   │
│  │ registration via x402.  │  │                        │   │
│  │                        │  │ npx mcp-remote          │   │
│  │ [Submit →]              │  │  onchainai.xyz/mcp      │   │
│  │                        │  │      [Copy]             │   │
│  └────────────────────────┘  └────────────────────────┘   │
│                                                                │
│  Categories                                                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│  │ ◇ Bridge │ │ ◇ Swap   │ │ ◇ Wallet │ │ ◇ Payments│         │
│  │ 32       │ │ 84       │ │ 49       │ │ 91       │          │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘          │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│  │ ◇ Lending│ │ ◇ Staking│ │ ◇ Trading│ │ ◇ NFT    │          │
│  │ 45       │ │ 38       │ │ 188      │ │ 49       │          │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘          │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│  │ ◇ Data   │ │ ◇ Dev    │ │ ◇ Identity│ │ ◇ Govern │         │
│  │ 136      │ │ 60       │ │ 37       │ │ 22       │          │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘          │
│  ┌──────────┐ ┌──────────┐                                   │
│  │ ◇ Social │ │ ◇ AI     │                                   │
│  │ 15       │ │ Agent 98 │                                   │
│  └──────────┘ └──────────┘                                   │
│                                                                │
└────────────────────────────────────────────────────────────────┘

─── 스크롤 시 왼쪽 사이드바 필터 + 리스트 ─────────────────────

┌──────────────┬─────────────────────────────────────────────────┐
│ Sidebar       │  Sort: [HOT ↓] [New] [Comments]    1,342 tools   │
│              │  ─────────────────────────────────────────────── │
│ (sticky,     │                                                 │
│  independent │  ┌────┐  Zapper MCP      [Verified] [MCP] [x402]│
│  scroll)     │  │logo│  60+ chain portfolio data                 │
│ ▸ Function    │  │ img│  Zapper · 3d ago · proprietary           │
│   Bridge     │  └────┘  ETH BASE OP ARB +56  ★ 340  comments 12│
│   Lending    │         $ npx mcp-remote https://mcp.zapper.xyz  │
│   Staking    │           [Copy]                                 │
│   Trading    │  ─────────────────────────────────────────────── │
│   NFT        │  ┌────┐  Coinbase AgentKit [Verified][SDK][x402]│
│   Data       │  │logo│  Agent wallet + onchain actions          │
│   Dev Tools  │  │ img│  Coinbase · 5d ago · Apache-2.0        │
│   Identity   │  └────┘  ETH BASE SOL        ★ 1,200 comments 24│
│   Governance │         $ npm i @coinbase/agentkit  [Copy]      │
│   Social     │  ─────────────────────────────────────────────── │
│   AI Agent   │  ┌────┐  BOB Gateway CLI  [Verified] [CLI]      │
│              │  │logo│  Bitcoin ↔ EVM bridge CLI                │
│ ▸ Asset Class │  │ img│  BOB Collective · 1d ago · MIT         │
│   Crypto     │  └────┘  BTC ETH BASE BNB +8  ★ 125  comments 3 │
│   RWA        │         $ npm i -g @gobob/gateway-cli  [Copy]   │
│   Derivatives│  ─────────────────────────────────────────────── │
│   Stablecoins│  ┌────┐  tiny.place       [Community] [Platform]│
│              │  │logo│  AI agent social economy                  │
│ ▸ Actor       │  │ img│  tiny.place · 2w ago                   │
│   Human      │  └────┘  BASE                  ★ 42   comments 5 │
│   AI Agent   │         https://tiny.place       [Visit]        │
│              │  ─────────────────────────────────────────────── │
│ ▸ Type        │  ...more (Product Hunt style, HOT order)        │
│   MCP        │                                                 │
│   CLI        │  [Load more ▼]                                  │
│   SDK        │                                                 │
│   API        │                                                 │
│   Skill      │                                                 │
│   x402       │                                                 │
│              │                                                 │
│ ▸ Status      │                                                 │
│   Verified   │                                                 │
│   Official   │                                                 │
│   Community  │                                                 │
│              │                                                 │
│ ▸ Chain       │                                                 │
│   Ethereum   │                                                 │
│   Bitcoin    │                                                 │
│   Solana     │                                                 │
│   Base       │                                                 │
│   Arbitrum   │                                                 │
│   Optimism   │                                                 │
│   Polygon    │                                                 │
│   BNB        │                                                 │
│   Avalanche  │                                                 │
│   ...more    │                                                 │
└──────────────┴─────────────────────────────────────────────────┘
```

**홈페이지 구조**:
1. **상단 네비게이션** — 로고, Submit, GitHub 링크
2. **소개 + 검색 섹션** — 슬로건, 사이트 설명, 검색바
3. **등록 유도 카드** (2개):
   - **좌: "Submit a Tool"** — "Register your MCP, CLI, SDK, API. Free, verified badges. x402 tools pay registration via x402." + [Submit →]
     > 수익 모델: 일반 등록 무료, x402 툴 등록은 x402로 등록료 결제 (예: $5/등록 또는 $10/검증배지).
   - **우: "OnchainAI MCP"** — "Connect your agent to search 1,342 tools." + MCP 엔드포인트 주소 + 복사 버튼
   - **데스크톱**: 2개 나란히 (가로 배치)
   - **모바일**: 세로 스택 (1개씩 위아래, 카드 전체 너비)
4. **카테고리 그리드** — 14개 기능 카테고리 (이모지 대신 단색 SVG 아이콘)
   - **데스크톱**: 5열 그리드, 16px gap. 14개 = 5+5+4, 마지막 줄 4개는 **좌측 정렬** (flex-start)
   - **태블릿**: 4열 그리드. 14개 = 4+4+4+2, 마지막 줄 2개 좌측 정렬
   - **모바일**: 2열 그리드. 14개 = 2×7, 여백 균등
5. **왼쪽 사이드바 + 리스트** — 3축 필터(기능/자산유형/주체) + 타입/상태/체인 필터, HOT 정렬

> 커뮤니티 등록 카드는 소개 섹션에서 눈에 띄게 배치 → 콜드스타트 해소 유도
> 카테고리 그리드 클릭 → 사이드바 해당 기능 필터 + 리스트로 스크롤
> 사이드바 각 섹션(▸) 클릭 시 펼침/접침
> HOT = stars + 최근 커밋 가중치로 인기순 정렬 (상단 정렬 드롭다운)

---

## 3. 도구 리스트 페이지 (`/tools`)

### 홈페이지와의 관계

**`/tools` = 홈페이지의 소개 섹션만 뺀 버전**. 같은 사이드바 + 리스트 구조.

| | 홈 (`/`) | 리스트 (`/tools`) |
|---|---|---|
| 소개 섹션 | 있음 (슬로건, 검색, 등록 카드, 카테고리 그리드) | 없음 |
| 검색바 | 소개 섹션 내 (풀 너비) | 정렬바 옆 (검색 아이콘 → 인라인 검색 입력) |
| 사이드바 필터 | 있음 | 있음 (동일) |
| 도구 리스트 | 있음 (HOT 순) | 있음 (정렬 선택 가능) |
| 정렬 드롭다운 | 리스트 상단 고정 | 리스트 상단 고정 (동일 위치) |

> 홈에서 카테고리 그리드 클릭 → `/tools?function=bridge`로 이동
> 홈에서 스크롤 다운 → 사이드바 + 리스트가 자연스럽게 이어짐 (별도 페이지 전환 아님)

```
┌────────────────────────────────────────────────────────────────┐
│  [🔍 Search...]  Sort: [HOT ↓] [New] [Comments]    1,342 tools  │  ← 검색바 + 정렬 드롭다운 (리스트 상단 고정)
│  ────────────────────────────────────────────────────────────── │
│                                                                │
│  ┌────┐  BOB Gateway CLI               [Verified] [CLI]        │
│  │logo│  Bitcoin ↔ EVM bridge CLI                              │  ← 카드 클릭 시 오른쪽 미리보기 패널
│  │ img│  BOB Collective · 1d ago · MIT                         │
│  └────┘  BTC ETH BASE BNB +8     ★ 125    comments 3          │
│          $ npm i -g @gobob/gateway-cli              [Copy]     │
│  ────────────────────────────────────────────────────────────── │
│  ┌────┐  Zapper MCP                  [Verified] [MCP] [x402]   │
│  │logo│  60+ chain portfolio data                              │
│  │ img│  Zapper · 3d ago · proprietary                         │
│  └────┘  ETH BASE OP ARB +56    ★ 340    comments 12          │
│          $ npx mcp-remote https://mcp.zapper.xyz   [Copy]      │
│  ────────────────────────────────────────────────────────────── │
│  ┌────┐  Solana Agent Kit            [Verified] [SDK]         │
│  │logo│  40+ Solana protocol actions                           │
│  │ img│  SendAI · 1w ago · Apache-2.0                          │
│  └────┘  SOL                              ★ 153    comments 8  │
│          $ npm i @sendaifun/solana-agent-kit        [Copy]     │
│  ────────────────────────────────────────────────────────────── │
│  ...                                                           │
└────────────────────────────────────────────────────────────────┘
```

> 카드 클릭 → 오른쪽 미리보기 패널 (5.9절). 인라인 펼치기는 제거됨.

### 모바일 카드 디자인

모바일(<768px)에서는 카드 내 정보를 간소화하고 레이아웃을 세로로 재배치.

```
┌────────────────────────────────────┐
│ ┌────┐  BOB Gateway CLI            │
│ │logo│  [Verified] [CLI]           │  ← 배지: flex-wrap, 줄바꿈 허용
│ │ img│  Bitcoin ↔ EVM bridge CLI   │
│ └────┘  BOB Collective · 1d ago    │  ← 라이선스 생략 (상세에서만)
│         BTC ETH BASE +8            │  ← 체인: 3개만 +N 접기
│         ★ 125  comments 3          │  ← install 명령어 생략 (모바일 카드)
└────────────────────────────────────┘
```

**모바일 카드 규칙**:
- install 명령어: 모바일 카드에서 **생략** (바텀 시트에서만 표시)
- 체인 배지: **상위 3개만 표시 + "+N" 접기** (예: BTC ETH BASE +8)
- 배지: `flex-wrap: wrap` 줄바꿈 허용 (한 줄에 안 들어가면 다음 줄)
- 라이선스(MIT/Apache): 생략, 상세/바텀 시트에서만
- 카드 패딩: 16px (데스크톱 24px에서 축소)
- 로고: 40x40px (데스크톱 48px에서 축소)
- 줄바꿈: 설명 2줄까지 truncate (ellipsis), 상세에서 전체
- 터치 타겟: 카드 전체 높이 ≥ 72px (44px 최소 + 패딩)

### 미리보기 패널 vs 상세 페이지

**두 가지 지원**:

1. **리스트에서 카드 클릭** → 오른쪽 미리보기 패널 (즉시, 같은 페이지, 5.9절 참조)
2. **도구 이름 더보기 링크** → `/tools/bob-gateway-cli` 상세 페이지 (직접 링크/SEO용)

> 인라인 펼치기는 미리보기 패널로 대체. VS Code 스타일.

**미리보기 패널 내용**:
```
  ┌────┐  BOB Gateway CLI
  │logo│  [Verified] [Official] [CLI] [Bridge]
  └────┘  [Crypto] [Human]                      ← 3축: 자산유형 + 주체

  ★ 125  comments 3  updated 1d ago  bookmark [☆]
  ────────────────────────────────────────────

  Description:
    BOB Gateway moves Bitcoin across 11+ chains in one click.
    AI agent docs included. v0.2.0 adds send command (BTC/EVM direct transfer, PSBT support).

  Install:
    [Claude] [Cursor] [Generic]
    $ npm install -g @gobob/gateway-cli    [Copy]

  Chains: [Bitcoin] [Ethereum] [Base] [BNB] [Arbitrum] +6

  Links:
    [GitHub ↗ 125★]  [Docs ↗]  [Homepage ↗]  [npm ↗]

  Trust:
    ✓ Verified · Official Team (BOB Collective) · Active (1d ago commit)
    License: MIT

  Comments (3):
    user1: "BTC swap on Base is fast, gas is cheap"
    user2: "agent docs are a bit lacking..."
    [Write a comment...]
                                         [✕ Close]
```

---

## 4. 도구 상세 페이지 (`/tools/:slug`)

미리보기 패널의 전체 버전. 더 많은 정보 + 댓글 스레드. 직접 링크/SEO용.

**레이아웃**: 본문 `max-width: 720px; margin: 0 auto` (대화면에서 가독성 유지). 좌우 여백 32px.

```
┌────────────────────────────────────────────────────────────────┐
│  ← All Tools                                                   │  ← 뒤로 가기: 필터 쿼리 보존 (?function=bridge&sort=hot)
│                                                                │
│  ┌────┐                                                        │
│  │logo│  BOB Gateway CLI                                       │
│  │ img│  [Verified] [Official: BOB Collective] [CLI] [Bridge] │
│  └────┘  [Crypto] [Human]                                     │  ← 3축: 자산유형 + 주체
│                                                                │
│  ★ 125    comments 3    updated 1d ago    bookmark [☆]        │
│  ────────────────────────────────────────────────────────────── │
│                                                                │
│  Description                                                   │
│  BOB Gateway moves Bitcoin across 11+ chains in one click.    │
│  AI agent docs included. v0.2.0 adds send command (BTC/EVM    │
│  direct transfer, PSBT support).                              │
│                                                                │
│  Install                                                       │
│  [Claude] [Cursor] [Generic]                                   │
│  ┌────────────────────────────────────────────────┐           │
│  │ npm install -g @gobob/gateway-cli              │ [Copy]    │
│  └────────────────────────────────────────────────┘           │
│  or:                                                           │
│  ┌────────────────────────────────────────────────┐           │
│  │ npx @gobob/gateway-cli                         │ [Copy]    │
│  └────────────────────────────────────────────────┘           │
│                                                                │
│  Chains                                                        │
│  [Bitcoin] [Ethereum] [Base] [BNB] [Arbitrum] [Optimism]      │
│  [Polygon] [Avalanche] [Linea] [Mode] [BOB] +1                │
│  ↑ flex-wrap: wrap (모바일에서 자동 줄바꿈, 가로 스크롤 없음)   │
│                                                                │
│  Links                                                         │
│  [GitHub ↗ 125★]  [Docs ↗]  [Homepage ↗]  [npm ↗]            │
│                                                                │
│  Trust                                                         │
│  ✓ Verified badge                                             │
│  ✓ Official team: BOB Collective                              │
│  ✓ Active: 1d ago commit                                      │
│  ✓ GitHub 125 stars                                           │
│  License: MIT                                                  │
│                                                                │
│  Comments (3)                          [Sort: New ↓]          │
│  ────────────────────────────────────────────────────────────── │
│  [GH] alice · 2h ago                                            │  ← GitHub 인증, 닉네임
│  BTC swap on Base is fast, gas is cheap.                     │
│  ↑ 5    [Reply]                                               │
│                                                                │
│  [0x] crypto_dev · 1d ago                                       │  ← 지갑 인증, 닉네임
│  agent docs are a bit lacking...                              │
│  gateway-cli folder README is solid though.                   │
│  ↑ 2    [Reply]                                               │
│    [GH] bob · 20h ago                                           │
│      docs.gobob.xyz/gateway/agents page just added.          │
│      ↑ 1                                                      │
│                                                                │
│  [Write a comment...]                              [Post]     │  ← 미인증 시 클릭 → 로그인 모달
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

## 5. 핵심 UI 컴포넌트 명세

### 5.1 배지 (Badge)

> 배지 색상 정의는 6절 색상 시스템에 통합. 아래는 배지 종류만 나열.

```
[Verified]  [Official]  [MCP]  [CLI]  [SDK]  [API]  [Skill]  [x402]  [Community]
```

- Verified / Official: #1A1A1A 테두리, #F5F5F0 배경
- MCP / CLI / SDK / API / Skill: #D1D1D1 테두리, #FAFAFA 배경
- x402: #1A1A1A 테두리+배경, #FFFFFF 텍스트 (검정 반전)
- Community: #D1D1D1 테두리, 투명 배경

> AI가 만드는 과장된 배지 금지. 텍스트 + 얇은 테두리 + 옅은 배경만.

### 5.2 로고 이미지

- 각 도구의 **공식 로고** 표시 (크롤 시 GitHub org logo / favicon / 공식 홈페이지 로고 수집)
- 로고 없으면 **첫 글자 모노그램** (회색 원 + 흰 글자)
- 크기: 48x48px (리스트), 64x64px (상세)

### 5.3 복사 버튼

npm/GitHub 패턴. 명령어 옆에 작은 복사 아이콘.

```
$ npm install -g @gobob/gateway-cli    [⧉ Copy]
```

클릭 시: "Copied" 텍스트로 2초간 변경 (토스트 아님, 인라인).

**코드 블록 overflow 규칙**:
- 데스크톱 카드: `overflow-x: auto; white-space: nowrap` — 긴 명령어는 가로 스크롤 (줄바꿈 안 함)
- 미리보기 패널(400px): `overflow-x: auto; white-space: nowrap` — 패널 너비보다 긴 명령어는 가로 스크롤
- 상세 페이지: `overflow-x: auto; word-break: break-all` — 매우 긴 URL은 줄바꿈 허용
- 스크롤바: thin scrollbar (`scrollbar-width: thin`), #E5E5E5 색상
- 모바일 바텀 시트: `overflow-x: auto` — 동일하게 가로 스크롤

### 5.4 별 / 북마크

```
★ 125    →  GitHub stars (읽기 전용, 크롤 데이터)
☆        →  북마크 (로그인 시 클릭 가능, 개인 컬렉션)
```

북마크는 인증된 유저만. Supabase bookmarks 테이블 (5.6절 인증 필요).

### 5.5 카테고리 아이콘 (단색 SVG line icons, Lucide 기반)

```
Bridge          →  git-branch 아이콘
Swap            →  arrow-left-right 아이콘
Wallet          →  credit-card 아이콘
Payments        →  dollar-sign 아이콘
Lending         →  banknote 아이콘
Staking         →  lock 아이콘
Trading         →  trending-up 아이콘
NFT             →  image 아이콘
Data            →  bar-chart 아이콘
Dev Tools       →  terminal 아이콘
Identity        →  fingerprint 아이콘
Governance      →  vote 아이콘
Social          →  message-circle 아이콘
AI Agent        →  bot 아이콘
```

### 5.6 인증 시스템 (3-way 통합)

3가지 로그인 방식을 지원. 모두 Supabase Auth로 통합 관리.

```
┌──────────────────────────────────────────────────────┐
│              Sign in to OnchainAI                     │
│                                                      │
│  ┌──────────────────────────────────────────────┐   │
│  │  [GitHub icon]  Continue with GitHub          │   │  ← OAuth (개발자)
│  └──────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────┐   │
│  │  [Mail icon]   Continue with Email            │   │  ← 매직 링크 (일반)
│  └──────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────┐   │
│  │  [Wallet icon] Connect Wallet                 │   │  ← SIWX (크립토)
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  By signing in, you agree to our Terms & Privacy.   │
└──────────────────────────────────────────────────────┘
```

**인증 방식 상세**:

| 방식 | 구현 | 대상 | 표시 |
|---|---|---|---|
| GitHub OAuth | Supabase Auth GitHub provider | 개발자 | GitHub 아이콘 + 닉네임 + avatar |
| Email 매직링크 | Supabase Auth Email (magic link) | 일반 유저 | 이메일 아이콘 + 닉네임 + 모노그램 avatar |
| 지갑 (SIWX) | CAIP-122 Sign-In-With-X (x402 V2 extension) | 크립토 유저/AI agent | 지갑 아이콘 + 닉네임 + ENS/모노그램 avatar |

> 모든 방식 공통: 첫 로그인 시 **닉네임 + 프로필 설정 온보딩** (아래 참조).
> GitHub username은 기본값으로 프리필되지만 변경 가능.
> 지갑 주소는 기본값으로 0x1234...abc 형태지만 닉네임으로 변경 가능.
> 실명/지갑 주소/이메일 노출 없이 닉네임으로만 활동 가능 (익명성 보장).

### 첫 로그인 온보딩 (프로필 설정)

로그인 성공 후 첫 방문 시 프로필 설정 화면:

```
┌──────────────────────────────────────────────────────┐
│              Welcome to OnchainAI                     │
│                                                      │
│  Set up your profile. You can change this later.     │
│                                                      │
│  Nickname                                            │
│  ┌──────────────────────────────────────────────┐   │
│  │ alice                                         │   │  ← 필수, 중복 체크
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  Bio (optional)                                      │
│  ┌──────────────────────────────────────────────┐   │
│  │ Building crypto tooling. Interested in...     │   │  ← 선택, 200자 제한
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  Avatar (optional)                                   │
│  ┌────┐  [Upload] or use auto-generated monogram    │  ← GitHub avatar 자동, 지갑은 ENS/모노그램
│  │    │                                              │
│  └────┘                                              │
│                                                      │
│  [Skip for now]              [Save & Continue]       │
│                                                      │
└──────────────────────────────────────────────────────┘
```

**온보딩 규칙**:
- 닉네임: 필수, 2-20자, 영문/숫자/하이픈/언더바, 중복 불가 (실시간 체크)
- Bio: 선택, 최대 200자, 댓글 페이지에서는 미표시 (프로필 페이지에서만)
- Avatar: 선택, GitHub 유저는 자동 불러오기, 지갑 유저는 ENS avatar 또는 닉네임 첫 글자 모노그램
- [Skip for now]: 닉네임 미설정 시 자동 닉네임 부여 (예: `user-a3k9`), 나중에 변경 가능
- 프로필은 Settings 페이지에서 언제든 수정 가능

**익명성 보장**:
- GitHub 로그인: username이 아닌 닉네임으로 표시 (다르게 설정 가능)
- Email 로그인: 이메일 절대 노출 안 함, 닉네임만 표시
- 지갑 로그인: 지갑 주소 절대 노출 안 함, 닉네임만 표시
- 프로필 페이지에서도 인증 수단(이메일/지갑 주소)은 본인에게만 보임

**SIWX 지갑 인증 (x402 V2 연동)**:
- x402 V2의 Sign-In-With-X 확장 사용 (CAIP-122 기반)
- EVM (Ethereum, Base, Polygon 등) + Solana 지원
- 스마트 지갑 지원 (EIP-1271 / EIP-6492 — Coinbase Smart Wallet, Safe)
- Auth-only 라우트: 결제 없이 지갑 서명만으로 인증 (댓글/업보트/북마크용)
- x402 등록 결제 시 자동으로 인증 세션 생성 (이미 지갑 연결된 상태에서 결제 → 인증 완료)
- 서명 메시지: "Sign in to OnchainAI to comment, upvote, and bookmark tools"
- 세션 만료: 24시간 (서명 시간 기준)
- nonce: 서버에서 1회성 nonce 발급 (리플레이 공격 방지)

**인증별 권한 (모두 동일)**:
- 댓글 작성, 답글, 업보트
- 북마크 (개인 컬렉션)
- 도구 등록 (Submit)
- x402 등록 결제 (지갑 로그인 유저만 — 이미 지갑 연결됨)
- GitHub/Email 유저가 x402 등록 시: 별도 지갑 연결 단계 추가

**인증별 뱃지 (댓글/등록 표시)**:
```
GitHub 유저:   [GH] alice
Email 유저:    [Mail] bob
지갑 유저:     [0x] crypto_dev
```
> 닉네임으로 표시, 실제 username/이메일/지갑 주소 노출 안 함.
> 인증 방식 아이콘만 작게 표시 (신뢰도 참고용).

> 인증 없이 사이트 탐색(검색, 필터, 미리보기, 상세 페이지)은 자유로움.
> 댓글/업보트/북마크/등록 시에만 인증 필요.
> 미인증 상태에서 댓글 입력 필드 클릭 → 로그인 모달 팝업.

### 5.7 댓글 시스템

- **인증 필요**: 댓글 작성/업보트/답글은 5.6절 인증 필요 (미인증 시 로그인 모달)
- 스레드 답글 (1단계 깊이만, Hacker News보다 단순)
- 업보트 (▲) — 인증된 유저만, 중복 방지 (user_id + comment_id unique)
- 정렬: 최신 / 인기
- 댓글 작성자 표시: 인증 방식별 뱃지 + 닉네임 (5.6절 참조)
- MVP: Supabase comments 테이블 + Leptos server function

### 5.7 왼쪽 사이드바 필터 (3축 + 타입/상태/체인)

상단 탭 대신 왼쪽 세로 사이드바로 모든 필터를 그룹화.

```
▸ 기능        — Bridge, Swap, Wallet, Payments, Lending, Staking,
                Trading, NFT, Data, Dev Tools, Identity, Governance,
                Social, AI Agent
▸ 자산 유형    — Crypto, RWA, Derivatives, Stablecoins
▸ 주체        — Human, AI Agent
▸ 타입        — MCP, CLI, SDK, API, Skill, x402
▸ 상태        — Verified, Official, Community
▸ 체인        — Ethereum, Bitcoin, Solana, Base, Arbitrum, Optimism,
                Polygon, BNB, Avalanche, ... (더보기 펼침)
                ※ 체인 목록은 DB의 tools.chains에서 동적 집계
                  (등록된 툴이 있는 체인만 표시, 툴 수 옆에 표기)
                ※ 상위 9개는 툴 수 많은 순 고정, 나머지는 "더보기" 클릭 시 전체 펼침
```

- 각 섹션(▸) 클릭 시 하위 항목 펼침/접침
- 기본적으로 기능 + 체인 섹션은 펼침, 나머지는 접힘
- 항목 클릭 = 필터 토글 (다중 선택 가능, AND 조건)
- 활성 필터: #1A1A1A 텍스트 + 체크 표시
- 상단 정렬 드롭다운: HOT / 최신 / 코멘트순
- URL 쿼리로 필터 상태 동기화: `?function=bridge,swap&chain=ethereum&sort=hot`
- **사이드바 고정**: `position: sticky; top: 56px` (네비 높이 56px 아래부터), `height: calc(100vh - 56px)`, `overflow-y: auto` (사이드바만 독립 스크롤)
- **정렬바 고정**: 리스트 상단 정렬 드롭다운도 `position: sticky; top: 56px` (스크롤해도 정렬/카운트 보임)
- **z-index**: 네비(100) > 정렬바(90) > 사이드바(80) > 미리보기 패널(70)

### 5.8 사이드바 접기/펼치기 (VS Code 스타일)

```
펼침 상태 (기본):
┌────┬─────────────────────────────────────┐
│필터│  리스트                              │
│240px│                                     │
└────┴─────────────────────────────────────┘

접힘 상태:
┌──┬──────────────────────────────────────┐
│☰ │  리스트 (전체 너비)                   │
│40px│                                     │
└──┴──────────────────────────────────────┘
```

- 접기 버튼: 사이드바 상단 `☰` 아이콘 (또는 `◀`)
- 접힘 상태: 40px 폭, 아이콘만 표시 (기능/자산유형/주체/타입/상태/체인 아이콘)
- 아이콘 호버 시 툴팁으로 섹션명 표시
- 다시 클릭 시 펼침 (240px)
- 상태 저장: localStorage (사용자별 선호 유지)
- 모바일: 사이드바 기본 접힘, `☰` 클릭 시 풀스크린 오버레이

### 5.9 오른쪽 미리보기 패널 (VS Code 에디터 패널 스타일)

리스트에서 도구 카드 클릭 시, 오른쪽에서 미리보기 패널이 밀려나옴.
별도 페이지 이동 없이 즉시 상세 확인.

```
┌────┬──────────────────────┬──────────────────────┐
│필터│  리스트               │  미리보기             │
│    │                      │  (선택된 도구 상세)    │
│240px│                     │ 400px                │
└────┴──────────────────────┴──────────────────────┘

미리보기 패널이 열린 상태:
┌────┬────────────┬─────────────────────────────┐
│Side│ List (narrow)│ ┌────┐ BOB Gateway CLI       │
│bar │            │ │logo│ [Verified] [CLI]       │
│    │ ┌────┐     │ └────┘ [Bridge]               │
│    │ │BOB │◀   │                                │
│    │ │CLI │sel  │ ★ 125  comments 3  1d ago     │
│    │ └────┘     │ ───────────────────────────── │
│    │ ┌────┐     │                                │
│    │ │Zap │     │ Description                    │
│    │ │MCP │     │ BOB Gateway moves Bitcoin...  │
│    │ └────┘     │                                │
│    │ ┌────┐     │ Install                        │
│    │ │Sol │     │ [Claude] [Cursor] [Generic]    │
│    │ │SDK │     │ $ npm i -g @gobob/gateway-cli  │
│    │ └────┘     │   [Copy]                       │
│    │            │                                │
│    │            │ Chains                         │
│    │            │ [Bitcoin] [Ethereum] [Base]... │
│    │            │                                │
│    │            │ Links                          │
│    │            │ [GitHub ↗] [Docs ↗] [npm ↗]    │
│    │            │                                │
│    │            │ Trust                          │
│    │            │ ✓ Verified · Official · Active │
│    │            │                                │
│    │            │ Comments (3)                   │
│    │            │ user1: "BTC swap on Base..."   │
│    │            │ [Write a comment...]  [Post]   │
│    │            │                           [✕]  │
└────┴────────────┴─────────────────────────────┘
```

**동작 방식**:
- 리스트에서 도구 카드 클릭 → 오른쪽 패널 밀려나옴 (슬라이드, 200ms)
- 리스트는 축소되지만 계속 보임 (선택된 카드 하이라이트: #F5F5F0 배경 + #D1D1D1 테두리)
- 패널 닫기: `✕` 버튼 또는 패널 외부 클릭 또는 ESC
- 패널 내용 = 인라인 펼치기 내용과 동일 (설명, 설치, 체인, 링크, 신뢰, 댓글)
- 패널 너비: 400px (데스크톱 ≥1024px만. 태블릿/모바일은 5.10절 바텀 시트)
- 다른 도구 클릭 → 패널 내용만 교체 (슬라이드 없이, 즉시 교체)
- URL 동기화: `/tools?selected=bob-gateway-cli` (공유 가능)

**2가지 보기 모드 (데스크톱)**:
```
1. 리스트만 (기본):  필터 + 리스트 전체 너비
2. 리스트 + 미리보기:  필터 + 리스트(축소) + 미리보기 패널 (400px)
```

> 모바일/태블릿은 5.10절 바텀 시트로 대체 (별도 보기 모드 없음).

> 도구 카드 클릭 → 미리보기 패널 (즉시, 같은 페이지)
> 도구 상세 페이지(`/tools/:slug`)는 여전히 존재 (직접 링크/SEO용, 미리보기의 전체 버전)

### 5.10 모바일 미리보기 — 바텀 시트 (Bottom Sheet)

데스크톱의 오른쪽 패널은 모바일에서 **아래에서 위로 밀려올라오는 바텀 시트**로 대체.
화면이 좁아 3단(사이드바+리스트+패널)이 불가능하므로, 같은 데이터를 다른 프레젠테이션으로.

```
모바일 - 리스트만 (기본):
┌──────────────────────┐
│ OnchainAI    [Search] [≡] │
│──────────────────────│
│ Sort: [HOT]  1,342   │
│                      │
│ ┌────┐ BOB Gateway.. │
│ │logo│ [Verified][CLI]│
│ └────┘ 125  3 com    │
│──────────────────────│
│ ┌────┐ Zapper MCP    │
│ │logo│ [Verified][MCP]│
│ └────┘ 340  12 com   │
│──────────────────────│
│ ...                  │
└──────────────────────┘

모바일 - 카드 탭 → 바텀 시트 (60% 높이):
┌──────────────────────┐
│ OnchainAI    [Search] [≡] │  ← 리스트 뒤, 반투명 어둡게 (포커스 분리)
│──────────────────────│
│ ┌────┐ BOB Gateway.. │  ← 선택된 카드 (베이지 배경)
│ │logo│ 125           │
│ └────┘               │
│░░░░░░░░░░░░░░░░░░░░░░│  ← 바깥 영역 블러/디밍
│ ════════════════════ │  ← 드래그 핸들 (회색 막대)
│ BOB Gateway CLI       │  ← 바텀 시트 (아래서 밀려올라옴)
│ [Verified] [CLI]      │
│ [Crypto] [Human]     │
│                      │
│ Description:          │
│ BOB Gateway moves... │
│                      │
│ Install:              │
│ $ npm i -g @gobob... │
│   [Copy]             │
│                      │
│ [View full page →]   │  ← 상세 페이지로 이동
└──────────────────────┘

바텀 시트 위로 드래그 → 풀스크린:
┌──────────────────────┐
│ [Close]         [Open ↗] │  ← 닫기 / 전체 페이지
│──────────────────────│
│ BOB Gateway CLI       │
│ [Verified] [CLI]      │
│ 125  3 com  1d ago   │
│──────────────────────│
│                      │
│ (전체 상세 내용)       │
│ Description          │
│ Install              │
│ Chains               │
│ Links                │
│ Trust                │
│ Comments (3)         │
│                      │
└──────────────────────┘
```

**동작 방식**:
- 카드 탭 → 바텀 시트가 아래에서 60% 화면 높이로 밀려올라옴 (슬라이드 250ms ease-out)
- 위로 드래그 → 풀스크린으로 확장 (90% 높이, 상단에 Close + Open 버튼)
- 아래로 드래그 또는 시트 외부 영역 탭 → 닫기 (슬라이드 다운 200ms)
- 시트 외부 영역: 반투명 어둡게 (#1A1A1A 30% opacity) + 블러 (포커스 분리)
- 드래그 핸들: 상단 회색 막대 (40px 너비, 4px 높이, #D1D1D1)
- 시트 배경: #FFFFFF, 상단 12px 라운드 (위쪽 모서리만)
- 시트 내용: 미리보기 패널과 동일 (설명, 설치, 체인, 링크, 신뢰, 댓글)
- 60% 상태에서는 요약 (설명 + 설치 + "View full page" 버튼)
- 풀스크린 상태에서는 전체 내용 (댓글 포함)
- "View full page" / "Open" → `/tools/:slug` 상세 페이지로 이동

**바텀 시트 상태**:
```
1. 닫힘:        리스트만 표시
2. 피크 (60%):  요약 (설명, 설치, 체인, "View full page")
3. 풀스크린:    전체 내용 (댓글 포함, Close + Open 버튼)
```

> 데스크톱: 오른쪽 패널 (5.9절)
> 모바일/태블릿: 바텀 시트 (5.10절)
> 같은 데이터, 다른 프레젠테이션. 반응형으로 자동 전환.

---

## 6. 색상 시스템 (라이트 모드, 뉴트럴 톤 + 주황 포인트)

> DESIGN.md와 통일. 흰색/베이지/회색/검정 뉴트럴 + 주황(#E76F00) 단일 액센트.

```
배경:        #FFFFFF (본문) / #F5F5F0 (베이지 섹션) / #FAFAFA (호버)
텍스트:      #1A1A1A (본문) / #6B6B6B (보조) / #999999 (미약)
테두리:      #E5E5E5 (기본) / #D1D1D1 (강조)
액센트:      #E76F00 (주황) — CTAs, 포커스 링, 링크, 활성 필터 점
액센트 호버:  #D96400 (주황 어둡게) — 버튼 호버만

배지 (테두리 + 배경만, 텍스트는 본문색):
  Verified    →  #1A1A1A 테두리, #F5F5F0 배경
  Official    →  #1A1A1A 테두리, #F5F5F0 배경
  MCP/CLI/SDK →  #D1D1D1 테두리, #FAFAFA 배경
  API/Skill   →  #D1D1D1 테두리, #FAFAFA 배경
  x402        →  #1A1A1A 테두리, #1A1A1A 배경, #FFFFFF 텍스트 (대비만)
  Community   →  #D1D1D1 테두리, 투명 배경

카테고리 아이콘: 전부 #4B4B4B (단색 회색, 색상 구분 없음)
카테고리 그리드 카드: #FFFFFF 배경, #E5E5E5 테두리, 호버 시 #F5F5F0

사이드바:
  활성 필터:  #1A1A1A 텍스트 + 주황 4px 점 표시
  비활성:     #6B6B6B 텍스트
  섹션 헤더:  #1A1A1A 굵게

버튼:
  Primary:    #E76F00 배경, #FFFFFF 텍스트 — 1화면 1개만
  Secondary:  #FFFFFF 배경, #D1D1D1 테두리, #1A1A1A 텍스트
  Hover:      Primary → #D96400 / Secondary → #F5F5F0 배경

포커스 링:    #E76F00 2px — 입력 필드, 검색바, 버튼
링크:        #1A1A1A 본문 → #E76F00 호버 (밑줄)
```

> 주황(#E76F00)은 1화면 1곳만 (가장 중요한 CTA). 배경/보더/아이콘에 주황 사용 금지.
> 그라디언트 금지. 다중 액센트 색상 금지. 주황이 유일한 비 뉴트럴 색상.

### 카드 디자인 (Stripe 스타일)

```
일반 상태:
  배경:     #FFFFFF
  테두리:   1px solid #E5E5E5
  그림자:   0 1px 2px rgba(0,0,0,0.04)  — 아주 옅음, 거의 안 보임
  라운드:   8px

호버 상태:
  테두리:   1px solid #D1D1D1
  그림자:   0 2px 8px rgba(0,0,0,0.06)  — 살짝 떠오름
  배경:     #FFFFFF (유지)
  트랜지션: 200ms ease

선택 카드 상태 (미리보기 패널 열림):
  테두리:   1px solid #D1D1D1
  그림자:   0 2px 8px rgba(0,0,0,0.06)
  배경:     #F5F5F0 (베이지, 선택됨 표시)
```

> 테두리 + 미세 그림자로 세련됨. 호버 시 살짝 떠오르는 효과.
> 그림자는 거의 안 보일 정도로 옅게 — 과장 없음.

**데스크톱 카드 텍스트 overflow 규칙**:
- 배지 영역: `flex-wrap: wrap; gap: 4px` — 좁은 리스트에서 배지가 자동 줄바꿈
- 체인 배지: `flex-wrap: wrap` + 상위 5개 표시 + "+N" 접기 (데스크톱 기본 5개, 미리보기 패널 열 시 리스트 축소되면 3개)
- 도구 이름: `white-space: nowrap; overflow: hidden; text-overflow: ellipsis` — 한 줄, 넘치면 말줄임
- 설명: `display: -webkit-box; -webkit-line-clamp: 1; -webkit-box-orient: vertical; overflow: hidden` — 한 줄만, 상세에서 전체
- install 명령어: `overflow-x: auto` (위 5.3절 코드 블록 규칙 참조)
- 팀명 + 시간 + 라이선스: `white-space: nowrap` — 한 줄 고정, 메타데이터 라인

### 빈 상태 (검색 결과 없음)

```
┌──────────────────────────────────────────────────────┐
│                                                      │
│                    ◇ (gray icon, large)               │
│                                                      │
│              No results found.                        │
│                                                      │
│      Try a different keyword or adjust filters.      │
│                                                      │
│         [Clear filters]  [Submit a tool →]           │
│                                                      │
└──────────────────────────────────────────────────────┘
```

- 배경: #FAFAFA
- 아이콘: #999999, 48px
- 텍스트: #6B6B6B
- "Submit a tool" → 결과 없을 때 등록 유도 (콜드스타트 해소)

### 로딩 상태 (스켈레톤)

```
┌────┐  ████████████          [▓▓▓▓] [▓▓]
│ ▓▓ │  ████████████          ▓▓▓▓▓▓ ▓▓▓▓▓▓▓▓
└────┘  ██████ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
        $ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ [▓▓]
─────────────────────────────────────────────────────
┌────┐  ████████████          [▓▓▓▓] [▓▓]
│ ▓▓ │  ████████████          ▓▓▓▓▓▓ ▓▓▓▓▓▓▓▓
└────┘  ██████ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
        $ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ [▓▓]
```

- 스켈레톤 블록: #F5F5F0 배경, #E5E5E5 블록
- 애니메이션: shimmer (좌→우로 옅은 하이라이트 이동, 1.5s loop)
- 로고 자리: 회색 사각형
- 리스트 구조와 동일한 레이아웃으로 스켈레톤 표시

### 에러 상태 (서버 오류 / 크롤 실패)

```
┌──────────────────────────────────────────────────────┐
│                                                      │
│                    ◇ (gray icon, large)               │
│                                                      │
│              Failed to load data.                     │
│                                                      │
│      This may be a temporary issue. Please retry.    │
│                                                      │
│              [Retry]                                 │
│                                                      │
└──────────────────────────────────────────────────────┘
```

- 배경: #FAFAFA
- 아이콘: #999999, 48px
- 텍스트: #6B6B6B
- "Retry" 버튼: #1A1A1A 테두리, #FFFFFF 배경, 호버 시 #F5F5F0
- 자동 재시도: 3초 후 1회, 실패 시 수동 "Retry" 버튼 노출

---

## 7. 타이포그래피

```
본문:       Inter (sans-serif)
코드/명령어: JetBrains Mono (monospace)

데스크톱 (≥768px):
  H1 (페이지 타이틀):    28px, 700, line-height 1.2, letter-spacing -0.02em
  H2 (섹션):             20px, 600, line-height 1.3, letter-spacing -0.01em
  H3 (도구 이름):         16px, 600, line-height 1.4
  본문:                  14px, 400, line-height 1.6
  보조 텍스트:            13px, 400, line-height 1.5, #6B6B6B
  배지 텍스트:            11px, 600, line-height 1, uppercase, letter-spacing 0.06em
  코드:                  13px, 400, line-height 1.5

모바일 (<768px):
  H1:                    26px, 700, line-height 1.25, letter-spacing -0.01em
  H2:                    18px, 600, line-height 1.35
  H3:                    16px, 600, line-height 1.4
  본문:                  16px, 400, line-height 1.65  ← 데스크톱 14px에서 확대
  보조 텍스트:            14px, 400, line-height 1.55, #6B6B6B  ← 13px에서 확대
  배지 텍스트:            11px, 600 (동일, 배지는 작게 유지)
  코드:                  13px, 400 (동일, 가로 스크롤 허용)
```

**모바일 타이포그래피 규칙**:
- 본문 ≥ 16px (14px 절대 사용 금지)
- 줄높이 ≥ 1.65 (가독성)
- 자간: 헤드라인만 -0.01em, 본문은 0 (기본)
- 최소 터치 타겟: 44x44px (버튼, 리스트 항목, 필터 칩)
- 배지는 11px 유지 — 작아도 터치 대상 아님 (읽기 전용)

---

## 8. 추가 데이터베이스 스키마 (인증, 댓글, 북마크)

### users (Supabase Auth 확장)

> Supabase Auth가 auth.users 테이블 관리. 아래는 public 프로필 테이블.

```sql
CREATE TABLE profiles (
  id UUID PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
  auth_method TEXT NOT NULL,           -- 'github' | 'email' | 'siwx'
  nickname TEXT NOT NULL,              -- 닉네임 (필수, 2-20자, unique)
  bio TEXT,                            -- 자기소개 (선택, 200자 제한)
  avatar_url TEXT,                     -- GitHub avatar / 업로드 이미지 / ENS avatar
  wallet_address TEXT,                 -- SIWX 유저만 (0x...), 본인에게만 노출
  is_admin BOOLEAN DEFAULT false,      -- 슈퍼관리자 권한
  is_banned BOOLEAN DEFAULT false,     -- 밴 상태 (댓글/업보트/등록 권한 박탈)
  created_at TIMESTAMPTZ DEFAULT now(),
  updated_at TIMESTAMPTZ DEFAULT now()
);

-- 첫 가입자 자동 슈퍼관리자 (트리거)
CREATE OR REPLACE FUNCTION set_first_user_admin()
RETURNS TRIGGER AS $$
BEGIN
  IF (SELECT COUNT(*) FROM profiles) = 0 THEN
    NEW.is_admin := true;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_first_user_admin
  BEFORE INSERT ON profiles
  FOR EACH ROW EXECUTE FUNCTION set_first_user_admin();

CREATE UNIQUE INDEX idx_profiles_nickname ON profiles(nickname);
CREATE INDEX idx_profiles_wallet ON profiles(wallet_address) WHERE wallet_address IS NOT NULL;
CREATE INDEX idx_profiles_admin ON profiles(is_admin) WHERE is_admin = true;
```

### comments 테이블

```sql
CREATE TABLE comments (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  parent_id UUID REFERENCES comments(id) ON DELETE CASCADE,  -- 답글 (1단계)
  user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  content TEXT NOT NULL,
  upvotes INT DEFAULT 0,
  created_at TIMESTAMPTZ DEFAULT now()
  -- 작성자 닉네임/avatar는 profiles 테이블과 JOIN하여 표시
);

CREATE INDEX idx_comments_tool ON comments(tool_id);
CREATE INDEX idx_comments_parent ON comments(parent_id);
CREATE INDEX idx_comments_user ON comments(user_id);
```

### upvotes 테이블 (중복 방지)

```sql
CREATE TABLE upvotes (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  comment_id UUID NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  created_at TIMESTAMPTZ DEFAULT now(),
  UNIQUE(comment_id, user_id)          -- 1유저 1댓글 1업보트
);
```

### bookmarks 테이블

```sql
CREATE TABLE bookmarks (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  created_at TIMESTAMPTZ DEFAULT now(),
  UNIQUE(tool_id, user_id)
);
```

### siwx_sessions (지갑 인증 세션)

```sql
CREATE TABLE siwx_sessions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  wallet_address TEXT NOT NULL,        -- 지갑 주소
  chain_id TEXT NOT NULL,              -- CAIP-2 (eip155:8453, solana:...)
  nonce TEXT NOT NULL UNIQUE,          -- 1회성 nonce (리플레이 방지)
  signature TEXT NOT NULL,             -- 서명 원본
  expires_at TIMESTAMPTZ NOT NULL,     -- 만료 시간 (발급 + 24h)
  created_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_siwx_wallet ON siwx_sessions(wallet_address);
CREATE INDEX idx_siwx_nonce ON siwx_sessions(nonce);
```

### tool_logos (로고 캐싱)

```sql
-- tools 테이블에 컬럼 추가
ALTER TABLE tools ADD COLUMN logo_url TEXT;      -- 공식 로고 URL
ALTER TABLE tools ADD COLUMN logo_monogram TEXT; -- 로고 없을 때 첫 글자
```

---

## 9. 반응형

```
데스크톱 (≥1024px):  왼쪽 사이드바(240px) + 리스트, 카테고리 5열 그리드
                      검색바: 소개 섹션 내
                      미리보기: 오른쪽 패널 400px (5.9절)
태블릿 (768-1023px):  왼쪽 사이드바 접힘(햄버거 토글), 카테고리 4열
                      검색바: 소개 섹션 내
                      미리보기: 바텀 시트 (5.10절, 태블릿은 화면 좁아 패널 대신 시트)
모바일 (<768px):     사이드바 풀스크린 필터 패널, 카테고리 2열, 정렬 드롭다운
                      검색바: 상단 네비에 검색 아이콘 → 풀스크린 검색 오버레이
                      소개 섹션 간소화 (슬로건 + 검색만, 카드는 세로 스택)
                      미리보기: 바텀 시트 (5.10절, 60% 높이 → 드래그 시 풀스크린)
```

### 모바일 검색 흐름

```
상단 네비:
  OnchainAI              [🔍] [≡]

검색 아이콘 클릭 → 풀스크린 검색 오버레이:
  ┌────────────────────────────────┐
  │  [✕]                           │
  │                                │
  │  ┌──────────────────────────┐  │
  │  │ Search: asset tracking... │  │
  │  └──────────────────────────┘  │
  │                                │
  │  Popular searches:             │
  │  Bridge  Swap  Wallet  DeFi    │
  │                                │
  └────────────────────────────────┘

햄버거(≡) 클릭 → 풀스크린 필터 패널 (사이드바 내용)
```

### 모바일 사용 흐름 (Core Flow)

모바일은 화면이 좁아 한 번에 하나의 레이어만 표시. 모든 오버레이는 풀스크린.

```
1. 진입:        홈 (슬로건 + 검색 + 등록 카드 + 카테고리 그리드 + HOT 리스트)
                ↓
2. 검색:        [🔍] 탭 → 풀스크린 검색 오버레이 → 결과 리스트
                ↓
3. 필터:        [≡] 탭 → 풀스크린 필터 패널 → 적용 → 리스트 갱신
                ↓
4. 도구 확인:    카드 탭 → 바텀 시트 (60%) → 요약 확인
                ↓ (위로 드래그)
5. 상세:        바텀 시트 풀스크린 → 전체 내용 + 댓글
                ↓
6. 설치:        [Copy] 탭 → 클립보드 복사 → "Copied" 인라인
                ↓
7. 외부 이동:    [Open ↗] → 상세 페이지 또는 외부 링크
```

**모바일 레이어 규칙**:
- 동시에 열리는 오버레이는 1개만 (검색 OR 필터 OR 바텀 시트, 중복 불가)
- 오버레이 열릴 때 본문 스크롤 잠금 (`overflow: hidden` on body)
- 뒤로 가기(스와이프 백 / 버튼) → 열린 오버레이부터 닫기
- 모든 오버레이는 풀스크린 (바텀 시트만 예외, 60% → 풀스크린)

## 11. 관리자 패널 (Admin Dashboard)

> MVP 포함. 슈퍼관리자(첫 가입자)가 웹에서 사이트 전체 관리.
> 관리자 패널은 `/admin` 라우트, 일반 유저에게는 보이지 않음.

### 11.1 관리자 인증

- **첫 가입자 자동 슈퍼관리자**: profiles 테이블의 첫 번째 레코드 → `is_admin = true` 자동 부여
- 이후 가입자: 일반 유저 (`is_admin = false`)
- 관리자 패널 접근: `is_admin = true`인 유저만 `/admin` 라우트 접근 가능
- 네비게이션: 관리자에게만 네비에 [Admin] 링크 표시

### 11.2 관리자 패널 구조

```
/admin                → 대시보드 (통계 요약)
/admin/tools          → 도구 관리 (승인/거절/수정/삭제)
/admin/categories     → 카테고리 관리 (추가/수정/삭제)
/admin/users          → 유저 관리 (밴/관리자 권한 부여)
/admin/comments       → 댓글 관리 (삭제)
/admin/settings       → 사이트 설정 (슬로건, 검색 키워드 등)
/admin/crawler        → 크롤러 상태 (소스별 상태, 수동 실행)
```

### 11.3 대시보드 (`/admin`)

```
┌────────────────────────────────────────────────────────────────┐
│  Admin Dashboard                                               │
│                                                                │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│  │ Tools    │ │ Pending  │ │ Users    │ │ Comments │          │
│  │ 1,342    │ │ 8        │ │ 156      │ │ 423      │          │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘          │
│                                                                │
│  Pending Approvals                                             │
│  ──────────────────────────────────────────────────────────    │
│  ┌────┐ NewTool MCP  [Community] [MCP]    submitted 2h ago    │
│  │logo│ "Crypto price feed MCP server"                        │
│  └────┘ github.com/newtool/mcp                                │
│         [Approve] [Reject] [Edit] [View Detail]               │
│  ──────────────────────────────────────────────────────────    │
│  ...                                                           │
│                                                                │
│  Crawler Status                                               │
│  ──────────────────────────────────────────────────────────    │
│  CryptoSkill    ● OK    1,342 items   3h ago                  │
│  GitHub topics  ● OK    892 items     12m ago                 │
│  npm            ● OK    340 items     8m ago                  │
│  web3-mcp-hub   ● Error 50 items      12h ago   [Retry]       │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### 11.4 도구 관리 (`/admin/tools`)

```
┌────────────────────────────────────────────────────────────────┐
│  Tool Management                          [Search...]  [Filter] │
│  ──────────────────────────────────────────────────────────    │
│  Status: [All] [Pending] [Approved] [Rejected]                 │
│                                                                │
│  ┌────┐ BOB Gateway CLI  [Verified] [CLI]    ★ 125            │
│  │logo│ [Approved]  1d ago                                    │
│  └────┘                                                        │
│         [Edit] [Delete] [Badges] [View]                        │
│  ──────────────────────────────────────────────────────────    │
│  ┌────┐ NewTool MCP  [Community] [MCP]      [Pending]         │
│  │logo│ submitted 2h ago by alice                              │
│  └────┘                                                        │
│         [Approve] [Reject] [Edit] [Badges] [View]              │
│  ──────────────────────────────────────────────────────────    │
│  ...                                                           │
└────────────────────────────────────────────────────────────────┘
```

**도구 관리 작업**:
- **승인 (Approve)**: pending → approved, status를 community에서 verified/official로 변경 가능
- **거절 (Reject)**: pending → rejected, 거절 사유 입력 (등록자에게 표시)
- **수정 (Edit)**: 이름, 설명, function, asset_class, actor, type, chains, install_command, mcp_endpoint, repo_url, homepage, license, pricing, x402_price — 모든 필드 웹에서 직접 수정
- **배지 부여 (Badges)**: Verified/Official 배지 부여/제거, Official Team명 설정
- **삭제 (Delete)**: 도구 완전 삭제 (soft delete 옵션: status='deleted')
- **크롤 도구 수정**: 크롤러가 수집한 도구도 관리자가 임의 수정 가능 (override)

### 11.5 인라인 에디터 (빠른 수정)

도구 상세 페이지에서 관리자는 인라인 에디터로 필드를 직접 클릭하여 수정:

```
일반 유저가 보는 상세 페이지:
│  Description                                                   │
│  BOB Gateway moves Bitcoin across 11+ chains in one click.    │

관리자가 보는 상세 페이지:
│  Description                                                   │
│  BOB Gateway moves Bitcoin across 11+ chains in one click. [✎] │  ← 클릭 시 인라인 편집
│  ┌────────────────────────────────────────────────┐           │
│  │ BOB Gateway moves Bitcoin across 11+ chains... │ [Save]    │
│  └────────────────────────────────────────────────┘           │
```

- 모든 필드에 [✎] 아이콘 (관리자에게만 표시)
- 클릭 → 인라인 텍스트 편집 → [Save] → 즉시 DB 업데이트
- 페이지 새로고침 불필요 (Leptos 시그널로 즉시 반영)

### 11.6 카테고리 관리 (`/admin/categories`)

```
┌────────────────────────────────────────────────────────────────┐
│  Category Management                                           │
│                                                                │
│  Function (14)                                                │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                       │
│  │ Bridge   │ │ Swap     │ │ Wallet   │                       │
│  │ 32 tools │ │ 84 tools │ │ 49 tools │                       │
│  │ [Edit]   │ │ [Edit]   │ │ [Edit]   │                       │
│  └──────────┘ └──────────┘ └──────────┘                       │
│  ...                                                           │
│  [+ Add Category]                                             │
│                                                                │
│  Asset Class (4)                                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│  │ Crypto   │ │ RWA      │ │ Deriv.   │ │ Stable   │          │
│  │ [Edit]   │ │ [Edit]   │ │ [Edit]   │ │ [Edit]   │          │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘          │
│  [+ Add Category]                                             │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

- **추가**: 새 카테고리 이름 + 아이콘(Lucide) + slug
- **수정**: 이름, 아이콘, 표시 순서
- **삭제**: 도구가 연결된 카테고리는 삭제 불가 (먼저 이동 필요)

### 11.7 유저 관리 (`/admin/users`)

```
┌────────────────────────────────────────────────────────────────┐
│  User Management                          [Search...]          │
│  ──────────────────────────────────────────────────────────    │
│  [GH] alice      [Admin]   12 comments   5 bookmarks         │
│  [0x] crypto_dev           8 comments    3 bookmarks         │
│  [Mail] bob                2 comments    0 bookmarks         │
│  [GH] spammer    [Banned]  47 comments   0 bookmarks         │
│                                                                │
│  작업: [Ban] [Unban] [Make Admin] [Remove Admin] [Delete]     │
└────────────────────────────────────────────────────────────────┘
```

- **밴 (Ban)**: 댓글/업보트/북마크/등록 권한 박탈 (계정은 유지)
- **관리자 권한 부여/제거**: is_admin 토글
- **삭제 (Delete)**: 계정 + 모든 댓글/북마크 삭제 (soft delete 옵션)

### 11.8 댓글 관리 (`/admin/comments`)

```
┌────────────────────────────────────────────────────────────────┐
│  Comment Management                        [Search...]         │
│  ──────────────────────────────────────────────────────────    │
│  [GH] alice on BOB Gateway CLI · 2h ago                       │
│  "BTC swap on Base is fast..."                                │
│  [Delete] [Delete + Ban User]                                 │
│  ──────────────────────────────────────────────────────────    │
│  [0x] spammer on Zapper MCP · 5m ago                          │
│  "Buy $SHIB now! moon!!!"                                     │
│  [Delete] [Delete + Ban User]                                 │
│  ──────────────────────────────────────────────────────────    │
└────────────────────────────────────────────────────────────────┘
```

- **삭제**: 스팸/불법 댓글 삭제
- **삭제 + 밴**: 댓글 삭제 + 작성자 밴

### 11.9 사이트 설정 (`/admin/settings`)

```
┌────────────────────────────────────────────────────────────────┐
│  Site Settings                                                 │
│                                                                │
│  Site Name:    [OnchainAI                            ]          │
│  Slogan:       [Crypto tools, unified.               ]          │
│  Description:  [Discover, install, and share...     ]          │
│                                                                │
│  MCP Endpoint: [npx mcp-remote onchainai.xyz/mcp   ]          │
│                                                                │
│  Search Keywords (for crawler hints):                          │
│  [mcp-server, crypto-mcp, web3-mcp, blockchain-mcp, ...]      │
│  [+ Add keyword]                                              │
│                                                                │
│  Registration:                                                │
│  [x] Allow free registration                                 │
│  [x] Require approval for new tools                           │
│  [ ] Allow x402 paid registration                             │
│                                                                │
│  [Save Settings]                                              │
└────────────────────────────────────────────────────────────────┘
```

- 사이트 이름, 슬로건, 설명 웹에서 직접 수정 (코드 수정 불필요)
- MCP 엔드포인트 주소
- 크롤러 검색 키워드 추가/제거
- 등록 설정 (자유 등록 허용, 신규 도구 승인 필요 여부, x402 등록 허용)

### 11.10 크롤러 제어 (`/admin/crawler`)

```
┌────────────────────────────────────────────────────────────────┐
│  Crawler Control                                               │
│                                                                │
│  Source          Status   Items    Last Run    Next Run        │
│  ──────────────────────────────────────────────────────────    │
│  CryptoSkill     ● OK     1,342    3h ago      3h              │
│  GitHub topics   ● OK     892      12m ago     48m             │
│  npm             ● OK     340      8m ago      52m             │
│  web3-mcp-hub    ● Error  50       12h ago     —               │
│                                                                │
│  [Run Now] (각 소스별 수동 실행 버튼)                            │
│  [Retry Error] (에러 소스 재시도)                                │
│                                                                │
│  GitHub Stars Sync: ● OK  last: 15m ago   [Sync Now]          │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

- 각 크롤 소스별 상태 확인
- 수동 실행 버튼 (즉시 크롤 시작)
- 에러 소스 재시도
- GitHub star 동기화 수동 실행

### 11.11 관리자 UI 규칙

- **디자인**: 일반 사이트와 동일한 뉴트럴 + 주황 톤 (별도 테마 없음)
- **관리자 표시**: 네비게이션에 [Admin] 링크 (관리자에게만)
- **인라인 에디터**: 상세 페이지에서 [✎] 아이콘 (관리자에게만)
- **관리자 배지**: 관리자의 댓글/프로필에 [Admin] 배지 표시 (신뢰도)
- **모바일**: 관리자 패널은 데스크톱 우선 (모바일에서도 동작하지만 최적화는 데스크톱)
- **접근 제한**: `/admin/*` 라우트는 서버사이드에서 `is_admin` 체크, 비관리자 접근 시 404

모바일에서 바텀 시트 내 댓글 작성 시 가상 키보드가 올라오면 시트가 가려지는 문제 대응.

```
바텀 시트 풀스크린 상태 + 댓글 입력 필드 포커스:
  ┌──────────────────────┐
  │ [Close]        [Open ↗] │  ← 상단 고정 (sticky)
  │──────────────────────│
  │ (스크롤 가능 영역)     │
  │ ...상세 내용...        │
  │ ...댓글 목록...        │
  │──────────────────────│
  │ [Write a comment...]  │  ← 입력 필드: 키보드 위에 고정 (sticky bottom)
  │                  [Post]│
  └──────────────────────┘
         ↑ 가상 키보드
```

**키보드 대응 규칙**:
- 댓글 입력 필드 포커스 시: `position: sticky; bottom: 0`로 키보드 위에 고정
- 바텀 시트 내용 영역: `overflow-y: auto` + `-webkit-overflow-scrolling: touch`
- 키보드 올라올 때 시트 높이 조정: `dvh` (dynamic viewport height) 단위 사용
- 키보드 내려가면 시트 원래 높이로 복귀
- 입력 필드 터치 타겟: 44px 최소 높이

---

## 10. 핵심 인터랙션 정리

| 인터랙션 | 동작 | 구현 |
|---|---|---|
| 검색 | 입력 시 실시간 필터 (디바운스 200ms) | Leptos 시그널 + server function |
| 사이드바 필터 | 항목 클릭 시 토글(다중선택), 리스트 갱신 | URL 쿼리 + Leptos 시그널 |
| 사이드바 펼침 | 섹션(▸) 클릭 시 하위 항목 펼침/접침 | Leptos `<Show>` 컴포넌트 |
| 정렬 | 상단 드롭다운: HOT / 최신 / 코멘트순 | server function 쿼리 |
| 상세 펼치기 | 도구 카드 클릭 → 오른쪽 미리보기 패널 밀려나옴 (슬라이드 200ms) | Leptos `<Show>` + CSS transform |
| 미리보기 패널 닫기 | `✕` 버튼 / 패널 외부 클릭 / ESC | Leptos 시그널 |
| 미리보기 다른 도구 | 다른 카드 클릭 → 패널 내용만 즉시 교체 | Leptos 시그널 |
| 모바일 바텀 시트 | 카드 탭 → 아래서 60% 밀려올라옴, 드래그로 풀스크린 | Leptos + 터치 이벤트 |
| 바텀 시트 닫기 | 아래로 드래그 / 외부 탭 / Close 버튼 | Leptos 시그널 |
| 사이드바 접기 | `☰` 클릭 → 40px로 축소, localStorage 저장 | Leptos + localStorage |
| 복사 버튼 | 클립보드 복사 + "복사됨" 인라인 텍스트 | `navigator.clipboard` |
| 북마크 | 클릭 시 토글, 로컬 스토리지 저장 | Leptos + localStorage |
| 댓글 작성 | 폼 제출, 서버펑션, 낙관적 업데이트. **인증 필요** (미인증 시 로그인 모달) | Leptos server function + Supabase Auth |
| 업보트 | 클릭 시 카운트 +1 (중복 방지). **인증 필요** | Supabase upvotes 테이블 (unique constraint) |
| 북마크 | 클릭 시 토글. **인증 필요** | Supabase bookmarks 테이블 |
| 로그인 | GitHub OAuth / Email 매직링크 / SIWX 지갑 서명 | Supabase Auth + x402 SIWX extension |
| 첫 로그인 온보딩 | 닉네임 + bio + avatar 설정 (Skip 가능, 자동 닉네임 부여) | Leptos server function + profiles 테이블 |
| 프로필 수정 | Settings 페이지에서 닉네임/bio/avatar 변경 | Leptos server function |
| 카테고리 그리드 | 클릭 시 사이드바 해당 기능 필터 + 스크롤 | URL `/tools?function=bridge` |
| 도구 등록 | "등록하기" 클릭 → 등록 폼. 일반 무료, x402 툴은 등록 시 x402 결제 (등록료/검증배지) | x402 결제 연동 (MVP 이후) |
| OnchainAI MCP 연결 복사 | MCP 엔드포인트 주소 복사 | `navigator.clipboard` |
